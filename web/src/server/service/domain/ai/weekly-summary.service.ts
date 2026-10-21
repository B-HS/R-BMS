import { AI_TEXT_LIMIT, aiWeeklyOutputSchema, type AiEffort, type AiWeeklyOutput, type AiWeeklyView } from '@server/dto/ai'
import { clearLampFromId, NO_ASSIST_GRADE, type AssistGrade } from '@server/dto/common'
import {
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
    MILLIS_DECIMALS,
    PERCENT_DECIMALS,
    REFERENCE_RULE_VERSION,
    SCORE_FEATURE_ANALYSIS_VERSION,
    WEEKLY_LIST_MAX,
} from '@server/service/domain/ai/score-feature.constants'
import { isClearLampId } from '@server/service/domain/ai/table-progress'
import { roundRatio } from '@server/service/domain/analysis/chart-features'

export const AI_WEEKLY_FEATURE = 'weekly_summary'
export const AI_WEEKLY_PROMPT_VERSION = 2
export const AI_WEEKLY_MAX_OUTPUT_TOKENS = 8_000
export const AI_WEEKLY_EFFORT = 'low' satisfies AiEffort
export const AI_WEEKLY_HIGHLIGHTS_MAX = 6

const DAY_MS = 86_400_000
const WEEK_DAYS = 7
const WEEK_MS = WEEK_DAYS * DAY_MS
const MONDAY = 1
const SUNDAY = 0
const ISO_DATE_LENGTH = 10
const PERCENT = 100
const MICROS_PER_MILLI = 1000

export const AI_WEEKLY_SYSTEM_PROMPT = [
    'You write a short weekly progress summary for a rhythm game (BMS) player from the numbers of one UTC week (Monday to Sunday).',
    DIGEST_IS_DATA_RULE,
    '`metrics` holds this week (`week.*`) and the week before (`prev.*`); `lamp_changes` and `ex_changes` list charts whose best went up this week; `levels` groups this week by difficulty table level.',
    'If `week.in_progress` is true the week has not ended; say so instead of judging the whole week.',
    'In `lamp_changes` and `ex_changes`, `from_assist_grade` and `to_assist_grade` are the `assist_grade` of `from_lamp` and `to_lamp`.',
    ASSIST_GRADE_RULE,
    `Write at most ${AI_WEEKLY_HIGHLIGHTS_MAX} \`highlights\`, each with the \`key\` of the digest item it rests on, a short Korean \`title\`, one or two Korean sentences \`body\` and a \`tone\`.`,
    DIGEST_KEY_RULE,
    '`headline` is one Korean sentence; `next_week` one or two Korean sentences of what to try next week.',
].join('\n')

export const AI_WEEKLY_SPEC = {
    feature: AI_WEEKLY_FEATURE,
    analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_WEEKLY_PROMPT_VERSION,
    system: AI_WEEKLY_SYSTEM_PROMPT,
    schemaName: 'weekly_summary',
    schema: aiWeeklyOutputSchema,
    maxOutputTokens: AI_WEEKLY_MAX_OUTPUT_TOKENS,
    effort: AI_WEEKLY_EFFORT,
} satisfies AiFeatureSpec<typeof aiWeeklyOutputSchema>

/** One play of the week as the summary reads it. `maxExScore` is the chart's maximum EX (notes × 2 when the row has none). */
export type WeeklyScoreRow = {
    chartSha256: string
    chartMd5: string | null
    title: string
    clear: number
    assistGrade: AssistGrade
    exScore: number
    maxExScore: number
    playedAt: number
    fast: number
    slow: number
    judgedNotes: number
    scratchAuto: boolean
    autoOffset: boolean
    meanOffsetUs: number | null
    ruleVersion: number | null
}

export type PriorBestRow = { chartSha256: string; scratchAuto: boolean; clear: number; assistGrade: AssistGrade; exScore: number }

export type AiWeeklySummaryServiceDb = {
    /** The user's plays with `from <= played_at < to`, autoplay left out. */
    listScoresBetween: (params: { userId: string; from: number; to: number }) => Promise<WeeklyScoreRow[]>
    /** The best EX of each chart (and SCRATCH AUTO or not) at each lamp and gauge among the user's plays before `before`. */
    listPriorBests: (params: { userId: string; chartSha256s: readonly string[]; before: number }) => Promise<PriorBestRow[]>
}

type AiWeeklySummaryServiceDeps = {
    db: AiWeeklySummaryServiceDb
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
    levelLabelOf: (md5: string) => string | null
    now: () => number
}

const toIsoDate = (at: number) => new Date(at).toISOString().slice(0, ISO_DATE_LENGTH)

/** The start (Monday 00:00 UTC) of the ISO week holding `at`. */
export const weekStartOf = (at: number) => {
    const day = Math.floor(at / DAY_MS) * DAY_MS
    const weekday = new Date(day).getUTCDay()
    const sinceMonday = weekday === SUNDAY ? WEEK_DAYS - 1 : weekday - MONDAY
    return day - sinceMonday * DAY_MS
}

/**
 * The week a request names: `YYYY-MM-DD` of its Monday (UTC), or the current week when absent. Null for a date that is not a real
 * Monday or a week after the current one.
 */
export const resolveWeekStart = (week: string | undefined, now: number) => {
    const current = weekStartOf(now)
    if (week === undefined) return current
    const at = Date.parse(`${week}T00:00:00.000Z`)
    if (Number.isNaN(at) || toIsoDate(at) !== week || new Date(at).getUTCDay() !== MONDAY || at > current) return null
    return at
}

const roundTo = (value: number, decimals: number) => {
    const scale = 10 ** decimals
    return Math.round(value * scale) / scale
}

const exRateOf = (exScore: number, maxExScore: number) => (maxExScore > 0 ? roundTo((exScore / maxExScore) * PERCENT, PERCENT_DECIMALS) : null)

type LampWithGrade = { clear: number; assistGrade: AssistGrade }

const UNPLAYED_LAMP = { clear: 0, assistGrade: NO_ASSIST_GRADE } satisfies LampWithGrade

/** Whether `to` is a higher lamp than `from`: a higher lamp id, or the same id with a higher assist grade. */
const isLampHigher = (to: LampWithGrade, from: LampWithGrade) =>
    to.clear > from.clear || (to.clear === from.clear && to.assistGrade > from.assistGrade)

const bestKey = (chartSha256: string, scratchAuto: boolean) => `${chartSha256}|${scratchAuto ? 1 : 0}`

/** The best lamp (with its assist grade) and the best EX of each chart, and the first row it came from; plays with and without SCRATCH AUTO stay apart. */
const foldBests = <Row extends Pick<WeeklyScoreRow, 'chartSha256' | 'scratchAuto' | 'clear' | 'assistGrade' | 'exScore'>>(rows: readonly Row[]) => {
    const bests = new Map<string, { row: Row; clear: number; assistGrade: AssistGrade; exScore: number }>()
    for (const row of rows) {
        const key = bestKey(row.chartSha256, row.scratchAuto)
        const current = bests.get(key)
        const lamp = current !== undefined && !isLampHigher(row, current) ? current : row
        bests.set(key, {
            row: current?.row ?? row,
            clear: lamp.clear,
            assistGrade: lamp.assistGrade,
            exScore: Math.max(current?.exScore ?? 0, row.exScore),
        })
    }
    return bests
}

/**
 * The deterministic summary of one week (plan 5.10): plays, charts, days played, charts first played, charts whose best lamp went
 * up to a clear lamp (and the new clears among them), charts whose best EX went up, FAST/SLOW and the timing bias of reference-rule
 * plays, all against the best each chart had before the week. Plays with and without SCRATCH AUTO keep separate bests.
 */
export const summarizeWeek = (params: {
    rows: readonly WeeklyScoreRow[]
    priors: readonly PriorBestRow[]
    levelLabelOf: (md5: string) => string | null
}) => {
    const prior = foldBests(params.priors)
    const changes = [...foldBests(params.rows).entries()].map(([key, best]) => {
        const before = prior.get(key)
        const md5 = best.row.chartMd5
        const change = {
            key: `chart.${md5 ?? best.row.chartSha256}`,
            title: best.row.title,
            level: md5 ? params.levelLabelOf(md5) : null,
            from_lamp: clearLampFromId(before?.clear ?? 0),
            from_assist_grade: before?.assistGrade ?? NO_ASSIST_GRADE,
            to_lamp: clearLampFromId(best.clear),
            to_assist_grade: best.assistGrade,
            from_ex_rate: before ? exRateOf(before.exScore, best.row.maxExScore) : null,
            to_ex_rate: exRateOf(best.exScore, best.row.maxExScore),
            scratch_auto: best.row.scratchAuto,
        }
        const isLampUp = isClearLampId(best.clear) && isLampHigher(best, before ?? UNPLAYED_LAMP)
        return {
            change,
            isFirst: !before,
            isLampUp,
            isNewClear: isLampUp && !isClearLampId(before?.clear ?? 0),
            isExUp: before !== undefined && best.exScore > before.exScore,
            exGain: (change.to_ex_rate ?? 0) - (change.from_ex_rate ?? 0),
            toClear: best.clear,
        }
    })
    const fast = params.rows.reduce((sum, row) => sum + row.fast, 0)
    const slow = params.rows.reduce((sum, row) => sum + row.slow, 0)
    const timed = params.rows.filter((row) => row.meanOffsetUs !== null && !row.autoOffset && (row.ruleVersion ?? 0) >= REFERENCE_RULE_VERSION)
    const timedNotes = timed.reduce((sum, row) => sum + row.judgedNotes, 0)
    const weightedOffset = timed.reduce((sum, row) => sum + (row.meanOffsetUs ?? 0) * row.judgedNotes, 0)
    const lampUps = changes.filter((entry) => entry.isLampUp)
    const levelStats = new Map<string, { plays: number; lamp_ups: number }>()
    for (const row of params.rows) {
        const level = row.chartMd5 ? params.levelLabelOf(row.chartMd5) : null
        if (!level) continue
        const stat = levelStats.get(level) ?? { plays: 0, lamp_ups: 0 }
        levelStats.set(level, { ...stat, plays: stat.plays + 1 })
    }
    for (const entry of lampUps) {
        const stat = entry.change.level ? levelStats.get(entry.change.level) : undefined
        if (stat && entry.change.level) levelStats.set(entry.change.level, { ...stat, lamp_ups: stat.lamp_ups + 1 })
    }
    return {
        totals: {
            plays: params.rows.length,
            charts: new Set(params.rows.map((row) => row.chartSha256)).size,
            active_days: new Set(params.rows.map((row) => Math.floor(row.playedAt / DAY_MS))).size,
            first_plays: changes.filter((entry) => entry.isFirst).length,
            lamp_ups: lampUps.length,
            new_clears: changes.filter((entry) => entry.isNewClear).length,
            ex_ups: changes.filter((entry) => entry.isExUp).length,
            fast,
            slow,
            fast_share: fast + slow > 0 ? roundRatio(fast / (fast + slow)) : null,
            mean_offset_ms: timedNotes > 0 ? roundTo(weightedOffset / timedNotes / MICROS_PER_MILLI, MILLIS_DECIMALS) : null,
        },
        lamp_changes: lampUps
            .toSorted(
                (left, right) =>
                    right.toClear - left.toClear ||
                    right.change.to_assist_grade - left.change.to_assist_grade ||
                    left.change.title.localeCompare(right.change.title),
            )
            .slice(0, WEEKLY_LIST_MAX)
            .map((entry) => entry.change),
        ex_changes: changes
            .filter((entry) => entry.isExUp)
            .toSorted((left, right) => right.exGain - left.exGain || left.change.title.localeCompare(right.change.title))
            .slice(0, WEEKLY_LIST_MAX)
            .map((entry) => entry.change),
        levels: [...levelStats.entries()]
            .map(([level, stat]) => ({ key: `level.${level}`, level, ...stat }))
            .toSorted((left, right) => right.plays - left.plays || left.level.localeCompare(right.level))
            .slice(0, WEEKLY_LIST_MAX),
    }
}

/**
 * Keeps highlights that point at a digest item and fit (plan 5.10); with none left the answer is rejected. The headline and the
 * next-week sentence are cut to the summary length.
 */
export const refineWeeklyOutput = (output: AiWeeklyOutput, digest: unknown) => {
    const keys = collectDigestKeys(digest)
    const highlights = output.highlights
        .filter(
            (item) =>
                keys.has(item.key) &&
                fitsLine(item.title, AI_TEXT_LIMIT.INSIGHT_TITLE_MAX_CHARS) &&
                fitsLine(item.body, AI_TEXT_LIMIT.INSIGHT_BODY_MAX_CHARS),
        )
        .slice(0, AI_WEEKLY_HIGHLIGHTS_MAX)
        .map((item) => ({ ...item, title: item.title.trim(), body: item.body.trim() }))
    if (highlights.length === 0) return null
    return {
        headline: cutText(output.headline, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
        highlights,
        next_week: cutText(output.next_week, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS),
    }
}

export const weeklySubjectKey = (weekStart: number) => `week:${toIsoDate(weekStart)}`

const metricsOf = (prefix: string, totals: ReturnType<typeof summarizeWeek>['totals']) =>
    Object.entries(totals).map(([name, value]) => ({ key: `${prefix}.${name}`, value }))

export const createAiWeeklySummaryService = (deps: AiWeeklySummaryServiceDeps) => {
    const summarize = async (userId: string, from: number) => {
        const rows = await deps.db.listScoresBetween({ userId, from, to: from + WEEK_MS })
        const priors =
            rows.length > 0
                ? await deps.db.listPriorBests({ userId, chartSha256s: [...new Set(rows.map((row) => row.chartSha256))], before: from })
                : []
        return summarizeWeek({ rows, priors, levelLabelOf: deps.levelLabelOf })
    }

    const build = async (params: { userId: string; week?: string }) => {
        const now = deps.now()
        const start = resolveWeekStart(params.week, now)
        if (start === null) return null
        const [current, previous] = await Promise.all([summarize(params.userId, start), summarize(params.userId, start - WEEK_MS)])
        const week = { start: toIsoDate(start), end: toIsoDate(start + WEEK_MS - DAY_MS), in_progress: now < start + WEEK_MS }
        return {
            start,
            week,
            current,
            previous,
            digest: {
                week: { key: 'week', ...week },
                metrics: [...metricsOf('week', current.totals), ...metricsOf('prev', previous.totals)],
                lamp_changes: current.lamp_changes,
                ex_changes: current.ex_changes,
                levels: current.levels,
            },
        }
    }

    return {
        /** The deterministic week summary and the latest stored AI summary of that week. Nothing is generated ahead of time. */
        view: async (params: { userId: string; week?: string }) => {
            const built = await build(params)
            if (!built) return null
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: AI_WEEKLY_SPEC,
                userId: params.userId,
                subjectKey: weeklySubjectKey(built.start),
                digest: built.digest,
            })
            return {
                week: built.week,
                totals: built.current.totals,
                previous: built.previous.totals,
                lamp_changes: built.current.lamp_changes,
                ex_changes: built.current.ex_changes,
                levels: built.current.levels,
                notice: built.current.totals.plays === 0 ? { need: 'week_plays' } : null,
                ai,
            } satisfies AiWeeklyView
        },

        /** Writes the week summary with the user's AI when the user opens it and presses the button; a week with no play makes no call. */
        interpret: async (params: { userId: string; week?: string }) => {
            const built = await build(params)
            if (!built) return null
            if (built.current.totals.plays === 0) return notEnoughData('week_plays')
            return runAiFeature(deps.runner, {
                ...AI_WEEKLY_SPEC,
                userId: params.userId,
                subjectKey: weeklySubjectKey(built.start),
                digest: built.digest,
                refine: (output) => refineWeeklyOutput(output, built.digest),
            })
        },
    }
}

export type AiWeeklySummaryService = ReturnType<typeof createAiWeeklySummaryService>
