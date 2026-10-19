import { AI_TEXT_LIMIT, type AiEffort, type AiInsight } from '@server/dto/ai'
import { aiReportOutputSchema, type AiReportOutput, type AiReportView } from '@server/dto/ai-play'
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
import {
    PLAY_FEATURE_ANALYSIS_VERSION,
    PLAY_LANE_BIAS_TOP,
    PLAY_REPORT_MIN_PLAYS,
    PLAY_REPORT_PREFERENCE_MIN_DIFF,
    PLAY_REPORT_REPEAT_PLAYS,
    PLAY_REPORT_TOP_ITEMS,
} from '@server/service/domain/ai/play-feature.constants'
import { exRatioOf, fastShareOf, isShownTag, playsWithMetrics, poolGroups, usToMs } from '@server/service/domain/ai/play-digest'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import { FRONTIER_DIGEST_LEVELS } from '@server/service/domain/ai/score-feature.constants'
import {
    findFrontierIndex,
    readTableProgress,
    type ChartTagLoad,
    type ProgressRecords,
    type ProgressTable,
} from '@server/service/domain/ai/table-progress'
import { REPORT_RECENT_PLAYS, SP7_LANE_NAMES } from '@server/service/domain/analysis/analysis-constants'
import { roundRatio } from '@server/service/domain/analysis/chart-features'
import { PATTERN_TAGS } from '@server/service/domain/analysis/pattern-tags'
import { EVIDENCE, TAG_LEANING, aggregateTagEvidence } from '@server/service/domain/analysis/sample'
import type { AnalyzedPlay, TraceService } from '@server/service/domain/trace/trace.service'

export const AI_PLAY_REPORT_FEATURE = 'play_report'
export const AI_PLAY_REPORT_PROMPT_VERSION = 2
export const AI_PLAY_REPORT_MAX_OUTPUT_TOKENS = 12_000
export const AI_PLAY_REPORT_EFFORT = 'low' satisfies AiEffort
export const AI_PLAY_REPORT_SUBJECT = 'recent'

const TAG_KEY_PREFIX = 'tag.'

export const AI_PLAY_REPORT_SYSTEM_PROMPT = [
    "You write a rhythm game (BMS, 7 keys and a scratch) player's report from the server's analysis of their recent plays.",
    DIGEST_IS_DATA_RULE,
    'A tag residual is, play by play, the EX ratio of the notes with that pattern tag less the whole play\'s, so a negative residual is a pattern the player loses EX on whatever the chart. `strengths` and `weaknesses` hold only tags the evidence supports (grade "strong").',
    `Write at most ${PLAY_REPORT_TOP_ITEMS} \`strengths\` and ${PLAY_REPORT_TOP_ITEMS} \`weaknesses\`, each pointing at a tag of the input list of the same name by its \`key\`; never call a tag that is only in \`uncertain_tags\` or \`insufficient_tags\` a strength or a weakness.`,
    `Write at most ${PLAY_REPORT_TOP_ITEMS} \`habits\` from \`habits\` and \`lanes\`, and practice steps in \`practice_focus\` (\`key\` of the item the step trains, one Korean sentence \`action\`).`,
    'Every `ex_ratio` is (2 PGREAT + GREAT) / (2 notes) over the notes that took a judgement; `notes` and `plays` say how much data stands behind a figure.',
    "The habit and lane figures are signed. `mean_ms` (timing bias) is the mean timing error of PGREAT and GREAT notes in milliseconds: positive means the notes were hit early (FAST), negative means late (SLOW). `mean_delta` (fatigue) is the EX ratio of the last third of a play less that of its first third, averaged over plays: negative means the player loses accuracy late in a play. A lane `deviation` is that lane's `ex_ratio` less the player's overall EX ratio: negative means the lane is worse than the player's average. `fast_share` is the share of the lane's GREAT to BAD judgements that were early (FAST): above 0.5 mostly early, below 0.5 mostly late. A `share` is the share of the analysed plays.",
    'Lane figures use the lanes as played after lane options, and the hands are the default assignment (scratch and keys 1-3 left, 4 shared, 5-7 right); say so when you talk about hands.',
    'In `caveats` write in Korean what the data cannot tell, always naming the tags in `uncertain_tags` and `insufficient_tags` when there are any.',
    DIGEST_KEY_RULE,
    'All text is Korean; `headline` is one sentence.',
].join('\n')

export const AI_PLAY_REPORT_SPEC = {
    feature: AI_PLAY_REPORT_FEATURE,
    analysisVersion: PLAY_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_PLAY_REPORT_PROMPT_VERSION,
    system: AI_PLAY_REPORT_SYSTEM_PROMPT,
    schemaName: 'player_report',
    schema: aiReportOutputSchema,
    maxOutputTokens: AI_PLAY_REPORT_MAX_OUTPUT_TOKENS,
    effort: AI_PLAY_REPORT_EFFORT,
} satisfies AiFeatureSpec<typeof aiReportOutputSchema>

export type AiPlayReportServiceDb = {
    /** FAST and SLOW of the account's newest score rows (autoplay left out), at most `limit`: what a player with no trace still has. */
    sumRecentFastSlow: (params: { userId: string; limit: number }) => Promise<{ plays: number; fast: number; slow: number }>
}

type AiPlayReportServiceDeps = {
    db: AiPlayReportServiceDb
    plays: Pick<TraceService, 'recentAnalyzedPlays'>
    loadTable: (tableId: string | undefined) => ProgressTable | null
    loadRecords: (params: { tableId: string; userId: string }) => Promise<ProgressRecords | null>
    loadChartTags: (md5s: readonly string[]) => Promise<ChartTagLoad>
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
}

const densityLowOf = (bucket: string) => Number.parseInt(bucket, 10)

const mean = (values: readonly number[]) => (values.length > 0 ? values.reduce((sum, value) => sum + value, 0) / values.length : null)

/** Mean tag shares of the charts in `md5s` that have shares, by tag. */
const meanShares = (md5s: readonly string[], shares: ChartTagLoad['shares']) => {
    const known = md5s.flatMap((md5) => {
        const share = shares.get(md5)
        return share ? [share] : []
    })
    return new Map(PATTERN_TAGS.filter(isShownTag).map((tag) => [tag, mean(known.map((share) => share[tag] ?? 0)) ?? 0]))
}

/**
 * Plan 5.1's deterministic report from the newest analysed plays (`plays.measured` of them have per-note figures, the 7-key single
 * plays, and only those count as the sample): tag residuals with their grade (plan 4.4, 4.6), accuracy by lane as
 * played and by note density, the mean fatigue, the timing bias of PGREAT and GREAT, how often each lane option was used, and the
 * patterns of the charts the player keeps choosing (charts played PLAY_REPORT_REPEAT_PLAYS times or more against every played chart
 * whose pattern joined, `layout: "original"`).
 */
export const computePlayReport = (plays: readonly AnalyzedPlay[], tagLoad: ChartTagLoad) => {
    const measured = playsWithMetrics(plays)
    const evidence = aggregateTagEvidence(
        measured.map(({ play, metrics }) => ({ scratchAuto: play.summary.scratch_auto, overall: metrics.overall, byTag: metrics.by_tag })),
    )
    const tags = PATTERN_TAGS.flatMap((tag) => {
        const row = evidence[tag]
        return row && row.notes > 0 ? [{ key: `${TAG_KEY_PREFIX}${tag}`, tag, ...row }] : []
    })
    const lanePools = poolGroups(measured.map(({ metrics }) => metrics.by_lane))
    const lanes = SP7_LANE_NAMES.flatMap((lane) => {
        const pool = lanePools.get(lane)
        return pool ? [{ key: `lane.${lane}`, name: lane, notes: pool.notes, ex_ratio: exRatioOf(pool), fast_share: fastShareOf(pool) }] : []
    })
    const density = [...poolGroups(measured.map(({ metrics }) => metrics.by_density)).entries()]
        .toSorted(([left], [right]) => densityLowOf(left) - densityLowOf(right))
        .map(([bucket, pool]) => ({ key: `density.${bucket}`, name: bucket, notes: pool.notes, ex_ratio: exRatioOf(pool) }))
    const fatigueDeltas = measured.flatMap(({ metrics }) => (metrics.fatigue.delta === null ? [] : [metrics.fatigue.delta]))
    const timingNotes = measured.reduce((sum, { metrics }) => sum + metrics.timing.notes, 0)
    const timingSum = measured.reduce((sum, { metrics }) => sum + metrics.timing.mean_us * metrics.timing.notes, 0)
    const optionCounts = Map.groupBy(plays, (play) => play.score.random)
    const options = [...optionCounts.entries()]
        .toSorted(([, left], [, right]) => right.length - left.length)
        .map(([option, inOption]) => ({ key: `option.${option}`, option, plays: inOption.length, share: roundRatio(inOption.length / plays.length) }))

    const chartPlays = Map.groupBy(
        plays.flatMap((play) => (play.summary.pattern.md5 ? [play.summary.pattern.md5] : [])),
        (md5) => md5,
    )
    const repeated = [...chartPlays.entries()].flatMap(([md5, times]) => (times.length >= PLAY_REPORT_REPEAT_PLAYS ? [md5] : []))
    const allShares = meanShares([...chartPlays.keys()], tagLoad.shares)
    const repeatShares = meanShares(repeated, tagLoad.shares)
    const preferred =
        repeated.length === 0
            ? []
            : [...allShares.entries()]
                  .map(([tag, all]) => {
                      const repeat = repeatShares.get(tag) ?? 0
                      return { key: `pref.${tag}`, tag, repeat_share: roundRatio(repeat), all_share: roundRatio(all), diff: roundRatio(repeat - all) }
                  })
                  .filter((row) => row.diff >= PLAY_REPORT_PREFERENCE_MIN_DIFF)
                  .toSorted((left, right) => right.diff - left.diff)
                  .slice(0, PLAY_REPORT_TOP_ITEMS)

    return {
        plays: { analyzed: plays.length, measured: measured.length, from: plays.at(-1)?.playedAt ?? null, to: plays[0]?.playedAt ?? null },
        tags,
        lanes,
        density,
        fatigue: { key: 'fatigue', plays: fatigueDeltas.length, mean_delta: fatigueDeltas.length > 0 ? roundRatio(mean(fatigueDeltas) ?? 0) : null },
        timing: { key: 'timing.bias', notes: timingNotes, mean_ms: timingNotes > 0 ? usToMs(timingSum / timingNotes) : null },
        options,
        preferred,
    }
}

export type PlayReport = ReturnType<typeof computePlayReport>

const overallExRatioOf = (report: PlayReport) => {
    const notes = report.lanes.reduce((sum, lane) => sum + lane.notes, 0)
    const weighted = report.lanes.reduce((sum, lane) => sum + (lane.ex_ratio ?? 0) * lane.notes, 0)
    return notes > 0 ? weighted / notes : null
}

/**
 * What the AI is given (plan 5.1): the strong strengths and weaknesses (at most PLAY_REPORT_TOP_ITEMS each), the other tags by name
 * only so they cannot be pointed at, the habits, the lanes that stray furthest from the player's own EX ratio, and the lamp levels
 * around the frontier.
 */
export const toPlayReportDigest = (report: PlayReport, lamps: AiReportView['lamps']) => {
    const strong = report.tags.filter((tag) => tag.grade === EVIDENCE.STRONG)
    const pick = ({ key, tag, residual, ex_ratio, notes, plays }: PlayReport['tags'][number]) => ({ key, tag, residual, ex_ratio, notes, plays })
    const overall = overallExRatioOf(report)
    const laneBias = report.lanes
        .map((lane) => ({ ...lane, deviation: lane.ex_ratio === null || overall === null ? 0 : roundRatio(lane.ex_ratio - overall) }))
        .toSorted((left, right) => Math.abs(right.deviation) - Math.abs(left.deviation))
        .slice(0, PLAY_LANE_BIAS_TOP)
    const frontierIndex = lamps ? (findFrontierIndex(lamps.levels) ?? 0) : 0
    return {
        plays: { key: 'plays', count: report.plays.measured },
        strengths: strong
            .filter((tag) => tag.leaning === TAG_LEANING.STRENGTH)
            .toSorted((left, right) => right.residual - left.residual)
            .slice(0, PLAY_REPORT_TOP_ITEMS)
            .map(pick),
        weaknesses: strong
            .filter((tag) => tag.leaning === TAG_LEANING.WEAKNESS)
            .toSorted((left, right) => left.residual - right.residual)
            .slice(0, PLAY_REPORT_TOP_ITEMS)
            .map(pick),
        uncertain_tags: report.tags.filter((tag) => tag.grade === EVIDENCE.UNCERTAIN).map((tag) => tag.tag),
        insufficient_tags: report.tags.filter((tag) => tag.grade === EVIDENCE.INSUFFICIENT).map((tag) => tag.tag),
        habits: [report.timing, report.fatigue, ...report.options.slice(0, 1), ...report.density.slice(-1)],
        lanes: laneBias.map(({ key, name, ex_ratio, deviation, fast_share }) => ({ key, lane: name, ex_ratio, deviation, fast_share })),
        preferred: report.preferred,
        lamps: lamps ? lamps.levels.slice(Math.max(0, frontierIndex - FRONTIER_DIGEST_LEVELS), frontierIndex + FRONTIER_DIGEST_LEVELS + 1) : [],
    }
}

export type PlayReportDigest = ReturnType<typeof toPlayReportDigest>

const keepInsights = (items: readonly AiInsight[], keys: ReadonlySet<string>) =>
    items
        .filter(
            (item) =>
                keys.has(item.key) &&
                fitsLine(item.title, AI_TEXT_LIMIT.INSIGHT_TITLE_MAX_CHARS) &&
                fitsLine(item.body, AI_TEXT_LIMIT.INSIGHT_BODY_MAX_CHARS),
        )
        .slice(0, PLAY_REPORT_TOP_ITEMS)
        .map((item) => ({ ...item, title: item.title.trim(), body: item.body.trim() }))

/**
 * Keeps what the digest backs. A strength or weakness that names a tag outside the strong list of its side rejects the whole answer
 * (plan 4.6: an `insufficient` tag in the weakness list fails the check); other items with an unknown key or that do not fit are
 * dropped. Caveats are required when the digest has uncertain or insufficient tags, and an answer with nothing left is rejected.
 */
export const refinePlayReportOutput = (output: AiReportOutput, digest: PlayReportDigest) => {
    const keys = collectDigestKeys(digest)
    const strengthKeys = new Set(digest.strengths.map((tag) => tag.key))
    const weaknessKeys = new Set(digest.weaknesses.map((tag) => tag.key))
    const isTagKey = (key: string) => key.startsWith(TAG_KEY_PREFIX)
    if (output.strengths.some((item) => isTagKey(item.key) && !strengthKeys.has(item.key))) return null
    if (output.weaknesses.some((item) => isTagKey(item.key) && !weaknessKeys.has(item.key))) return null
    const caveats = output.caveats.map((caveat) => cutText(caveat, AI_TEXT_LIMIT.REASON_MAX_CHARS)).filter((caveat) => caveat !== '')
    const hasDoubtfulTags = digest.uncertain_tags.length > 0 || digest.insufficient_tags.length > 0
    if (hasDoubtfulTags && caveats.length === 0) return null
    const strengths = keepInsights(output.strengths, strengthKeys)
    const weaknesses = keepInsights(output.weaknesses, weaknessKeys)
    const habits = keepInsights(output.habits, keys)
    const practiceFocus = output.practice_focus
        .filter((step) => keys.has(step.key) && fitsLine(step.action, AI_TEXT_LIMIT.REASON_MAX_CHARS))
        .slice(0, PLAY_REPORT_TOP_ITEMS)
        .map((step) => ({ key: step.key, action: step.action.trim() }))
    if (output.headline.trim() === '' || strengths.length + weaknesses.length + habits.length === 0) return null
    return {
        headline: cutText(output.headline, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        strengths,
        weaknesses,
        habits,
        practice_focus: practiceFocus,
        caveats,
    }
}

export const createAiPlayReportService = (deps: AiPlayReportServiceDeps) => {
    const build = async (userId: string) => {
        const { plays, stale, failed } = await deps.plays.recentAnalyzedPlays({ userId, limit: REPORT_RECENT_PLAYS })
        const md5s = [...new Set(plays.flatMap((play) => (play.summary.pattern.md5 ? [play.summary.pattern.md5] : [])))]
        const [tagLoad, scoreRows] = await Promise.all([deps.loadChartTags(md5s), deps.db.sumRecentFastSlow({ userId, limit: REPORT_RECENT_PLAYS })])
        const report = computePlayReport(plays, tagLoad)
        const table = deps.loadTable(undefined)
        const records = table ? await deps.loadRecords({ tableId: table.id, userId }) : null
        const lamps = table
            ? { table: { id: table.id, name: table.name, symbol: table.symbol }, levels: readTableProgress(table, records ?? {}).levels }
            : null
        return { report, stale, failed, lamps, scoreRows, digest: toPlayReportDigest(report, lamps) }
    }

    const noticeOf = (report: PlayReport) => (report.plays.measured < PLAY_REPORT_MIN_PLAYS ? { need: 'traced_plays' } : null)

    return {
        /** The deterministic report and the latest stored AI reading of it. No provider is called. */
        view: async (userId: string) => {
            const built = await build(userId)
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: AI_PLAY_REPORT_SPEC,
                userId,
                subjectKey: AI_PLAY_REPORT_SUBJECT,
                digest: built.digest,
            })
            return {
                ...built.report,
                plays: { ...built.report.plays, stale: built.stale, failed: built.failed },
                lamps: built.lamps,
                score_rows: built.scoreRows,
                notice: noticeOf(built.report),
                ai,
            } satisfies AiReportView
        },

        /** Asks the user's AI to write the report; fewer than PLAY_REPORT_MIN_PLAYS analysed plays make no call. */
        interpret: async (userId: string) => {
            const built = await build(userId)
            const notice = noticeOf(built.report)
            if (notice) return notEnoughData(notice.need)
            return runAiFeature(deps.runner, {
                ...AI_PLAY_REPORT_SPEC,
                userId,
                subjectKey: AI_PLAY_REPORT_SUBJECT,
                digest: built.digest,
                refine: (output) => refinePlayReportOutput(output, built.digest),
            })
        },
    }
}

export type AiPlayReportService = ReturnType<typeof createAiPlayReportService>
