import { AI_TEXT_LIMIT, aiRecommendOutputSchema, type AiEffort, type AiRecommendOutput, type AiRecommendView } from '@server/dto/ai'
import type { WeaknessProfileView } from '@server/dto/ai-play'
import {
    AI_OUTPUT_LANGUAGE_NAME,
    ASSIST_GRADE_RULE,
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
    DENSITY_OVER_PENALTY,
    DENSITY_OVER_RATIO,
    FOCUS_TAG_MIN_RATIO,
    FRONTIER_DIGEST_LEVELS,
    RECOMMEND_CANDIDATES,
    RECOMMEND_PICKS,
    SCORE_FEATURE_ANALYSIS_VERSION,
    WEAKNESS_FULL_SHARE,
    WEAKNESS_MATCH_TOP,
    WEAKNESS_MATCH_WEIGHT,
} from '@server/service/domain/ai/score-feature.constants'
import {
    challengeWindowOf,
    isDensityOver,
    rankChallengeCandidates,
    readTableProgress,
    toCandidate,
    weaknessShareOf,
    type ChartTagLoad,
    type ProgressRecords,
    type ProgressTable,
} from '@server/service/domain/ai/table-progress'
import type { WeaknessProfileService } from '@server/service/domain/ai/weakness-profile.service'
import { EVIDENCE } from '@server/service/domain/analysis/sample'
import { parseWeaknessKey } from '@server/service/domain/analysis/weakness-key'
import type { Locale } from '@shared/i18n/locale'

export const AI_RECOMMEND_FEATURE = 'recommend'
export const AI_RECOMMEND_PROMPT_VERSION = 3
export const AI_RECOMMEND_MAX_OUTPUT_TOKENS = 8_000
export const AI_RECOMMEND_EFFORT = 'low' satisfies AiEffort

export const buildAiRecommendSystemPrompt = (language: Locale) =>
    [
        'You help a rhythm game (BMS, 7 keys and a scratch) player choose which charts of a difficulty table to challenge next.',
        DIGEST_IS_DATA_RULE,
        'The server already chose the candidates in `candidates` and scored them; `frontier` is the level the player is working on and `levels` summarises nearby levels.',
        `Pick up to ${RECOMMEND_PICKS} charts from \`candidates\` only, in the order you would play them, each identified by its \`md5\`.`,
        `For each pick give one short ${AI_OUTPUT_LANGUAGE_NAME[language]} sentence \`reason\` and a \`focus_key\` naming the digest item the reason rests on (the chart key, a level key, \`frontier\`, or one of that candidate's \`focus_keys\`).`,
        'Chart `tags` describe the chart layout with no lane option (layout "original"), not how the player plays it.',
        "`weaknesses` are the strongest weaknesses of the player's profile that charts can train; a candidate's `focus_keys` names those its chart trains, and its score already counts them. `density` is the player's EX density limit; a candidate far beyond it scores lower.",
        ASSIST_GRADE_RULE,
        DIGEST_KEY_RULE,
        `\`summary\` is two ${AI_OUTPUT_LANGUAGE_NAME[language]} sentences at most about where the player stands in the table.`,
    ].join('\n')

export const aiRecommendSpecOf = (language: Locale) =>
    ({
        feature: AI_RECOMMEND_FEATURE,
        analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
        promptVersion: AI_RECOMMEND_PROMPT_VERSION,
        system: buildAiRecommendSystemPrompt(language),
        schemaName: 'chart_recommendations',
        schema: aiRecommendOutputSchema,
        maxOutputTokens: AI_RECOMMEND_MAX_OUTPUT_TOKENS,
        effort: AI_RECOMMEND_EFFORT,
    }) satisfies AiFeatureSpec<typeof aiRecommendOutputSchema>

export type TableFeatureDeps = {
    loadTable: (tableId: string | undefined) => ProgressTable | null
    loadRecords: (params: { tableId: string; userId: string }) => Promise<ProgressRecords | null>
    loadChartTags: (md5s: readonly string[]) => Promise<ChartTagLoad>
}

type AiRecommendServiceDeps = TableFeatureDeps & {
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
    /** The player's weakness profile (plan 2026-11-04 2.5); without it candidates get no weakness match and no density penalty. */
    profile?: Pick<WeaknessProfileService, 'view'>
}

/** The weaknesses candidates are matched against (plan 2026-11-04 2.6 feature 5): strong, able to pick charts, the first WEAKNESS_MATCH_TOP. */
export const matchableWeaknessesOf = (profile: Pick<WeaknessProfileView, 'weaknesses'> | null) =>
    (profile?.weaknesses ?? []).filter((item) => item.evidence === EVIDENCE.STRONG && item.selectable_for_folder).slice(0, WEAKNESS_MATCH_TOP)

/**
 * Keeps the AI's picks to the candidates the server chose (plan 5.5): a pick of another md5, an unknown `focus_key` or a reason
 * that does not fit on one line is dropped, and of the picks that pass, a chart's first one is kept and its repeats are dropped;
 * with no pick left the answer is rejected. A weakness key is a known `focus_key` only when it is one of that candidate's
 * `focus_keys` (plan 2026-11-04 2.6 feature 5).
 */
export const refineRecommendOutput = (
    output: AiRecommendOutput,
    digest: { candidates: readonly { md5: string; focus_keys?: readonly string[] }[] },
) => {
    const focusKeysOf = new Map(digest.candidates.map((candidate) => [candidate.md5, candidate.focus_keys ?? []]))
    const keys = collectDigestKeys(digest)
    const isKnownKey = (md5: string, key: string) => (parseWeaknessKey(key) === null ? keys.has(key) : (focusKeysOf.get(md5)?.includes(key) ?? false))
    const picks = output.picks
        .filter((pick) => focusKeysOf.has(pick.md5) && isKnownKey(pick.md5, pick.focus_key) && fitsLine(pick.reason, AI_TEXT_LIMIT.REASON_MAX_CHARS))
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
     * candidates with their pattern tags when the chart's pattern was uploaded. A candidate's score adds WEAKNESS_MATCH_WEIGHT times
     * its match with the profile's matchable weaknesses (each counts `min(1, share / WEAKNESS_FULL_SHARE) / WEAKNESS_MATCH_TOP`) and
     * takes DENSITY_OVER_PENALTY off a chart beyond the EX density limit (plan 2026-11-04 2.6); `focus_keys` are the weaknesses
     * whose share reaches FOCUS_TAG_MIN_RATIO. Null for an unknown table.
     */
    const buildDigest = async (params: { userId: string; tableId?: string; limit?: number }) => {
        const table = deps.loadTable(params.tableId)
        if (!table) return null
        const records = (await deps.loadRecords({ tableId: table.id, userId: params.userId })) ?? {}
        const progress = readTableProgress(table, records)
        const profile = deps.profile ? await deps.profile.view({ userId: params.userId }) : null
        const weaknesses = matchableWeaknessesOf(profile)
        const limitNps = profile?.density.limit_nps_ex ?? null
        const load = await deps.loadChartTags(challengeWindowOf(progress).map((chart) => chart.md5))
        const sharesOf = (md5: string) =>
            weaknesses.map((item) => ({
                key: item.key,
                share: weaknessShareOf(item.key, { tags: load.shares.get(md5), extras: load.extras?.get(md5), limitNps }),
            }))
        const bonusOf = (md5: string) => {
            const match = sharesOf(md5).reduce((sum, { share }) => sum + Math.min(1, share / WEAKNESS_FULL_SHARE), 0) / WEAKNESS_MATCH_TOP
            return WEAKNESS_MATCH_WEIGHT * match - (isDensityOver(load.extras?.get(md5), limitNps) ? DENSITY_OVER_PENALTY : 0)
        }
        const ranked =
            progress.frontierIndex === null
                ? []
                : rankChallengeCandidates({
                      charts: progress.charts,
                      levels: progress.levels,
                      frontierIndex: progress.frontierIndex,
                      limit: params.limit ?? RECOMMEND_CANDIDATES,
                      bonusOf: (chart) => bonusOf(chart.md5),
                  })
        const { shares: tags, staleMd5s } = load
        const candidates = ranked.map((entry) => ({
            ...toCandidate(entry.chart, tags),
            kind: entry.kind,
            score: entry.score,
            focus_keys: sharesOf(entry.chart.md5).flatMap(({ key, share }) => (share >= FOCUS_TAG_MIN_RATIO ? [key] : [])),
        }))
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
            profileKeys: weaknesses.map((item) => item.key),
            digest: {
                table: { id: table.id, name: table.name, symbol: table.symbol },
                frontier: progress.frontier ? { ...progress.frontier, key: 'frontier' } : null,
                levels: nearby,
                weaknesses: weaknesses.map(({ key, kind, value, unit, evidence, sample_level }) => ({
                    key,
                    kind,
                    value,
                    unit,
                    evidence,
                    sample_level,
                })),
                density: limitNps === null ? null : { key: 'density:limit', limit_nps_ex: limitNps, over_ratio: DENSITY_OVER_RATIO },
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
        view: async (params: { userId: string; tableId?: string; language: Locale }) => {
            const built = await buildDigest(params)
            if (!built) return null
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: aiRecommendSpecOf(params.language),
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
                profile_keys: built.profileKeys,
                notice: noticeOf(built),
                ai,
            } satisfies AiRecommendView
        },

        /** Asks the user's AI to order and explain the candidates. Called only from the user's button; no table record means no call. */
        interpret: async (params: { userId: string; tableId?: string; language: Locale }) => {
            const built = await buildDigest(params)
            if (!built) return null
            const notice = noticeOf(built)
            if (notice) return notEnoughData(notice.need)
            return runAiFeature(deps.runner, {
                ...aiRecommendSpecOf(params.language),
                userId: params.userId,
                subjectKey: recommendSubjectKey(built.table.id),
                digest: built.digest,
                refine: (output) => refineRecommendOutput(output, built.digest),
            })
        },
    }
}

export type AiRecommendService = ReturnType<typeof createAiRecommendService>
