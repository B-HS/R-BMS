import { AI_TEXT_LIMIT, aiRecommendOutputSchema, type AiEffort, type AiRecommendOutput, type AiRecommendView } from '@server/dto/ai'
import {
    DIGEST_IS_DATA_RULE,
    DIGEST_KEY_RULE,
    collectDigestKeys,
    cutText,
    fitsLine,
    notEnoughData,
    readStoredInterpretation,
    runAiFeature,
    type AiFeatureSpec,
} from '@server/service/domain/ai/ai-digest'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import {
    FRONTIER_DIGEST_LEVELS,
    RECOMMEND_CANDIDATES,
    RECOMMEND_PICKS,
    SCORE_FEATURE_ANALYSIS_VERSION,
} from '@server/service/domain/ai/score-feature.constants'
import {
    rankChallengeCandidates,
    readTableProgress,
    toCandidate,
    type ChartTagLoad,
    type ProgressRecords,
    type ProgressTable,
} from '@server/service/domain/ai/table-progress'

export const AI_RECOMMEND_FEATURE = 'recommend'
export const AI_RECOMMEND_PROMPT_VERSION = 1
export const AI_RECOMMEND_MAX_OUTPUT_TOKENS = 8_000
export const AI_RECOMMEND_EFFORT = 'low' satisfies AiEffort

export const AI_RECOMMEND_SYSTEM_PROMPT = [
    'You help a rhythm game (BMS, 7 keys and a scratch) player choose which charts of a difficulty table to challenge next.',
    DIGEST_IS_DATA_RULE,
    'The server already chose the candidates in `candidates` and scored them; `frontier` is the level the player is working on and `levels` summarises nearby levels.',
    `Pick up to ${RECOMMEND_PICKS} charts from \`candidates\` only, in the order you would play them, each identified by its \`md5\`.`,
    'For each pick give one short Korean sentence `reason` and a `focus_key` naming the digest item the reason rests on (the chart key, a level key or `frontier`).',
    'Chart `tags` describe the chart layout with no lane option (layout "original"), not how the player plays it.',
    DIGEST_KEY_RULE,
    '`summary` is two Korean sentences at most about where the player stands in the table.',
].join('\n')

export const AI_RECOMMEND_SPEC = {
    feature: AI_RECOMMEND_FEATURE,
    analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_RECOMMEND_PROMPT_VERSION,
    system: AI_RECOMMEND_SYSTEM_PROMPT,
    schemaName: 'chart_recommendations',
    schema: aiRecommendOutputSchema,
    maxOutputTokens: AI_RECOMMEND_MAX_OUTPUT_TOKENS,
    effort: AI_RECOMMEND_EFFORT,
} satisfies AiFeatureSpec<typeof aiRecommendOutputSchema>

export type TableFeatureDeps = {
    loadTable: (tableId: string | undefined) => ProgressTable | null
    loadRecords: (params: { tableId: string; userId: string }) => Promise<ProgressRecords | null>
    loadChartTags: (md5s: readonly string[]) => Promise<ChartTagLoad>
}

type AiRecommendServiceDeps = TableFeatureDeps & {
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
}

/**
 * Keeps the AI's picks to the candidates the server chose (plan 5.5): a pick of another md5, an unknown `focus_key` or a reason
 * that does not fit on one line is dropped, and of the picks that pass, a chart's first one is kept and its repeats are dropped;
 * with no pick left the answer is rejected.
 */
export const refineRecommendOutput = (output: AiRecommendOutput, digest: { candidates: readonly { md5: string }[] }) => {
    const allowed = new Set(digest.candidates.map((candidate) => candidate.md5))
    const keys = collectDigestKeys(digest)
    const picks = output.picks
        .filter((pick) => allowed.has(pick.md5) && keys.has(pick.focus_key) && fitsLine(pick.reason, AI_TEXT_LIMIT.REASON_MAX_CHARS))
        .filter((pick, index, valid) => valid.findIndex((other) => other.md5 === pick.md5) === index)
        .slice(0, RECOMMEND_PICKS)
        .map((pick) => ({ ...pick, reason: pick.reason.trim() }))
    if (picks.length === 0) return null
    return { summary: cutText(output.summary, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS), picks }
}

export const recommendSubjectKey = (tableId: string) => `table:${tableId}`

export const createAiRecommendService = (deps: AiRecommendServiceDeps) => {
    /**
     * The deterministic part of feature 5: the frontier level, the nearby level summary and up to RECOMMEND_CANDIDATES scored
     * candidates with their pattern tags when the chart's pattern was uploaded. Null for an unknown table.
     */
    const buildDigest = async (params: { userId: string; tableId?: string; limit?: number }) => {
        const table = deps.loadTable(params.tableId)
        if (!table) return null
        const records = (await deps.loadRecords({ tableId: table.id, userId: params.userId })) ?? {}
        const progress = readTableProgress(table, records)
        const ranked =
            progress.frontierIndex === null
                ? []
                : rankChallengeCandidates({
                      charts: progress.charts,
                      levels: progress.levels,
                      frontierIndex: progress.frontierIndex,
                      limit: params.limit ?? RECOMMEND_CANDIDATES,
                  })
        const { shares: tags, staleMd5s } = await deps.loadChartTags(ranked.map((entry) => entry.chart.md5))
        const candidates = ranked.map((entry) => ({ ...toCandidate(entry.chart, tags), kind: entry.kind, score: entry.score }))
        const nearby =
            progress.frontierIndex === null
                ? []
                : progress.levels.slice(
                      Math.max(0, progress.frontierIndex - FRONTIER_DIGEST_LEVELS),
                      progress.frontierIndex + FRONTIER_DIGEST_LEVELS + 1,
                  )
        const played = progress.charts.filter((chart) => chart.lampId > 0).length
        return {
            table: { id: table.id, name: table.name, symbol: table.symbol },
            progress,
            played,
            digest: {
                table: { id: table.id, name: table.name, symbol: table.symbol },
                frontier: progress.frontier ? { ...progress.frontier, key: 'frontier' } : null,
                levels: nearby,
                candidates,
            },
            patternCoverage: {
                with_features: candidates.filter((candidate) => tags.has(candidate.md5)).length,
                candidates: candidates.length,
                stale_features: candidates.filter((candidate) => staleMd5s.has(candidate.md5)).length,
            },
        }
    }

    const noticeOf = (built: NonNullable<Awaited<ReturnType<typeof buildDigest>>>) => {
        if (built.played === 0) return { need: 'table_records' }
        if (built.digest.candidates.length === 0) return { need: 'candidates' }
        return null
    }

    return {
        buildDigest,

        /** The digest view and the latest stored AI answer for it. No provider is called. */
        view: async (params: { userId: string; tableId?: string }) => {
            const built = await buildDigest(params)
            if (!built) return null
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: AI_RECOMMEND_SPEC,
                userId: params.userId,
                subjectKey: recommendSubjectKey(built.table.id),
                digest: built.digest,
            })
            return {
                table: built.table,
                frontier: built.progress.frontier,
                levels: built.progress.levels,
                candidates: built.digest.candidates,
                pattern_coverage: built.patternCoverage,
                notice: noticeOf(built),
                ai,
            } satisfies AiRecommendView
        },

        /** Asks the user's AI to order and explain the candidates. Called only from the user's button; no table record means no call. */
        interpret: async (params: { userId: string; tableId?: string }) => {
            const built = await buildDigest(params)
            if (!built) return null
            const notice = noticeOf(built)
            if (notice) return notEnoughData(notice.need)
            return runAiFeature(deps.runner, {
                ...AI_RECOMMEND_SPEC,
                userId: params.userId,
                subjectKey: recommendSubjectKey(built.table.id),
                digest: built.digest,
                refine: (output) => refineRecommendOutput(output, built.digest),
            })
        },
    }
}

export type AiRecommendService = ReturnType<typeof createAiRecommendService>
