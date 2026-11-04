import { CLEAR_LAMPS, MIN_CLEAR_LAMP_ID, NO_ASSIST_GRADE, clearLampToId, type AssistGrade, type ClearLamp } from '@server/dto/common'
import type { AiChartCandidate, AiLevelProgress } from '@server/dto/ai'
import type { TableRecordOutput } from '@server/dto/table-snapshot'
import { roundRatio, type readChartFeatureExtras } from '@server/service/domain/analysis/chart-features'
import {
    CANDIDATE_KIND_WEIGHT,
    CANDIDATE_SCORE_DECIMALS,
    CHART_TAG_MIN_RATIO,
    CHART_TAGS_NOT_SHOWN,
    CHART_TOP_TAGS,
    DENSITY_LIMIT_BAND,
    DENSITY_OVER_RATIO,
    FAILED_EX_WEIGHT,
    FOCUS_TAG_LEVELS_BELOW,
    FOCUS_TAG_MIN_RATIO,
    FRONTIER_CLEAR_RATE,
    FRONTIER_WINDOW_LEVELS,
    LEVEL_DISTANCE_WEIGHT,
    PERCENT_DECIMALS,
} from '@server/service/domain/ai/score-feature.constants'
import { parseWeaknessKey } from '@server/service/domain/analysis/weakness-key'

/** A difficulty table as the score features read it: levels in table order and one entry per chart. */
export type ProgressTable = {
    id: string
    name: string
    symbol: string
    levels: readonly string[]
    entries: readonly { md5: string; level: string; title: string; artist: string }[]
}

/** The player's best record per chart md5 (`tableSnapshotService.getRecords`). */
export type ProgressRecords = Readonly<Record<string, TableRecordOutput>>

/** Each tag's share of a chart's judged notes, from its stored pattern features (W3); absent when no pattern was uploaded. */
export type ChartTagShares = ReadonlyMap<string, Readonly<Record<string, number>>>

/**
 * What a chart's current features (ANALYSIS_VERSION 2) add to its tag shares: the sliding-window NPS, tag-pair shares and main BPM
 * band (read through `readChartFeatureExtras`), its soflan events and its length.
 */
export type ChartFeatureExtras = NonNullable<ReturnType<typeof readChartFeatureExtras>> & { soflanEvents: number; lengthMs: number }

/**
 * The tag shares of the charts that have a pattern, which of them come from features of an older analysis version, and the extra
 * features of the charts whose features are current; a stale or older chart has no extras (plan 2026-11-04 2.3).
 */
export type ChartTagLoad = { shares: ChartTagShares; staleMd5s: ReadonlySet<string>; extras?: ReadonlyMap<string, ChartFeatureExtras> }

export type ChartProgress = {
    md5: string
    title: string
    artist: string
    level: string
    levelIndex: number
    lamp: ClearLamp
    lampId: number
    assistGrade: AssistGrade
    exRate: number | null
    bp: number | null
    playedAt: number | null
}

const EX_PER_NOTE = 2
const PERCENT = 100
const FAILED_LAMP_ID = clearLampToId('Failed')
const HARD_LAMP_ID = clearLampToId('Hard')
const FULL_COMBO_LAMP_ID = clearLampToId('FullCombo')
const MILLIS_PER_SECOND = 1000
const KEY_SEPARATOR = ':'
const HIDDEN_TAGS = new Set<string>(CHART_TAGS_NOT_SHOWN)

const roundTo = (value: number, decimals: number) => {
    const scale = 10 ** decimals
    return Math.round(value * scale) / scale
}

/** EX score as a percent of the chart's maximum, two decimals; null with no notes to divide by. */
export const toExRatePercent = (exScore: number, totalNotes: number) =>
    totalNotes > 0 ? roundTo((exScore / (EX_PER_NOTE * totalNotes)) * PERCENT, PERCENT_DECIMALS) : null

export const isClearLampId = (lampId: number) => lampId >= MIN_CLEAR_LAMP_ID

/** Every chart of the table once (the first entry of a repeated md5), with the player's best record joined. */
export const toChartProgress = (table: ProgressTable, records: ProgressRecords) => {
    const levelIndex = new Map(table.levels.map((level, index) => [level, index]))
    const seen = new Set<string>()
    return table.entries.flatMap((entry) => {
        if (seen.has(entry.md5)) return []
        seen.add(entry.md5)
        const record = records[entry.md5]
        const lamp = record?.clear ?? CLEAR_LAMPS[0]
        return [
            {
                md5: entry.md5,
                title: entry.title,
                artist: entry.artist,
                level: entry.level,
                levelIndex: levelIndex.get(entry.level) ?? table.levels.length,
                lamp,
                lampId: clearLampToId(lamp),
                assistGrade: record?.assist_grade ?? NO_ASSIST_GRADE,
                exRate: record ? toExRatePercent(record.ex_score, record.total_notes) : null,
                bp: record ? record.minbp : null,
                playedAt: record ? record.played_at : null,
            } satisfies ChartProgress,
        ]
    })
}

/** Charts, plays and clears per level in table order (levels with no chart left out). */
export const summarizeLevels = (table: ProgressTable, charts: readonly ChartProgress[]) =>
    table.levels.flatMap((level) => {
        const inLevel = charts.filter((chart) => chart.level === level)
        if (inLevel.length === 0) return []
        const cleared = inLevel.filter((chart) => isClearLampId(chart.lampId)).length
        return [
            {
                key: `level.${level}`,
                level,
                charts: inLevel.length,
                played: inLevel.filter((chart) => chart.lampId > 0).length,
                cleared,
                hard_or_better: inLevel.filter((chart) => chart.lampId >= HARD_LAMP_ID).length,
                clear_rate: roundRatio(cleared / inLevel.length),
            } satisfies AiLevelProgress,
        ]
    })

/**
 * The level to challenge next: the one after the highest level whose clear rate reaches FRONTIER_CLEAR_RATE, the first level when
 * none does, the last when every level does. Plan 5.5 words it as the first level whose rate falls below the threshold; reading
 * from the highest cleared level keeps low levels a player never played from pulling the frontier down to the bottom of the table.
 */
export const findFrontierIndex = (levels: readonly AiLevelProgress[]) => {
    if (levels.length === 0) return null
    const solid = levels.findLastIndex((level) => level.clear_rate >= FRONTIER_CLEAR_RATE)
    if (solid < 0) return 0
    return Math.min(solid + 1, levels.length - 1)
}

const kindOf = (chart: ChartProgress) => {
    if (chart.lampId === 0) return 'unplayed' as const
    if (chart.lampId === FAILED_LAMP_ID) return 'failed' as const
    if (chart.lampId < HARD_LAMP_ID) return 'lamp_up' as const
    return null
}

/** The top tags of a chart (share of its judged notes), or none when the chart has no uploaded pattern. */
export const topTagsOf = (shares: Readonly<Record<string, number>> | undefined) =>
    Object.entries(shares ?? {})
        .filter(([tag, ratio]) => !HIDDEN_TAGS.has(tag) && ratio >= CHART_TAG_MIN_RATIO)
        .toSorted(([leftTag, left], [rightTag, right]) => right - left || leftTag.localeCompare(rightTag))
        .slice(0, CHART_TOP_TAGS)
        .map(([tag, ratio]) => ({ tag, ratio }))

export const toCandidate = (chart: ChartProgress, tags: ChartTagShares) =>
    ({
        key: `chart.${chart.md5}`,
        md5: chart.md5,
        title: chart.title,
        artist: chart.artist,
        level: chart.level,
        lamp: chart.lamp,
        assist_grade: chart.assistGrade,
        ex_rate: chart.exRate,
        bp: chart.bp,
        tags: topTagsOf(tags.get(chart.md5)),
    }) satisfies AiChartCandidate

const compareByTable = (left: ChartProgress, right: ChartProgress) =>
    left.levelIndex - right.levelIndex || left.title.localeCompare(right.title) || left.md5.localeCompare(right.md5)

/**
 * The next-challenge candidates (plan 5.5), in score order: charts within FRONTIER_WINDOW_LEVELS of the frontier that are failed,
 * unplayed, or cleared below HARD. The score is the kind's weight, less LEVEL_DISTANCE_WEIGHT per level away from the frontier,
 * plus FAILED_EX_WEIGHT times a failed chart's EX rate, plus `bonusOf` the chart (the weakness match and density penalty of plan
 * 2026-11-04 2.6, none when absent). Ties go by table order.
 */
export const rankChallengeCandidates = (params: {
    charts: readonly ChartProgress[]
    levels: readonly AiLevelProgress[]
    frontierIndex: number
    limit: number
    bonusOf?: (chart: ChartProgress) => number
}) => {
    const levelOrder = new Map(params.levels.map((level, index) => [level.level, index]))
    return params.charts
        .flatMap((chart) => {
            const index = levelOrder.get(chart.level)
            const kind = kindOf(chart)
            if (index === undefined || kind === null) return []
            const distance = Math.abs(index - params.frontierIndex)
            if (distance > FRONTIER_WINDOW_LEVELS) return []
            const closeness = kind === 'failed' && chart.exRate !== null ? (chart.exRate / PERCENT) * FAILED_EX_WEIGHT : 0
            const bonus = params.bonusOf?.(chart) ?? 0
            const score = roundTo(CANDIDATE_KIND_WEIGHT[kind] - LEVEL_DISTANCE_WEIGHT * distance + closeness + bonus, CANDIDATE_SCORE_DECIMALS)
            return [{ chart, kind, score }]
        })
        .toSorted((left, right) => right.score - left.score || compareByTable(left.chart, right.chart))
        .slice(0, params.limit)
}

/** The charts a challenge ranking can hold: those within FRONTIER_WINDOW_LEVELS of the frontier, before kinds and scores are given. */
export const challengeWindowOf = (progress: Pick<TableProgress, 'charts' | 'levels' | 'frontierIndex'>) => {
    if (progress.frontierIndex === null) return []
    const window = new Set(
        progress.levels
            .slice(Math.max(0, progress.frontierIndex - FRONTIER_WINDOW_LEVELS), progress.frontierIndex + FRONTIER_WINDOW_LEVELS + 1)
            .map((level) => level.level),
    )
    return progress.charts.filter((chart) => window.has(chart.level))
}

/**
 * The charts a practice folder or goal plan picks from (plan 5.6): FOCUS_TAG_LEVELS_BELOW levels below the frontier to
 * FRONTIER_WINDOW_LEVELS above it (the first level when there is no frontier), short of a FULL COMBO.
 */
export const practicePoolOf = (progress: Pick<TableProgress, 'charts' | 'levels' | 'frontierIndex'>) => {
    const frontier = progress.frontierIndex ?? 0
    const levels = new Set(
        progress.levels.slice(Math.max(0, frontier - FOCUS_TAG_LEVELS_BELOW), frontier + FRONTIER_WINDOW_LEVELS + 1).map((level) => level.level),
    )
    return progress.charts.filter((chart) => levels.has(chart.level) && chart.lampId < FULL_COMBO_LAMP_ID)
}

/** What a chart is matched against a weakness with: its tag shares, its current extra features, and the player's EX density limit. */
export type WeaknessShareInput = {
    tags: Readonly<Record<string, number>> | undefined
    extras: ChartFeatureExtras | undefined
    limitNps: number | null
}

/**
 * How much of a chart trains one weakness key (plan 2026-11-04 2.6 feature 5), 0 to 1: a tag's share of the judged notes for
 * `tag:` and `solo:`, the tag pair's share for `combo:`, 1 when the chart's 90th-percentile NPS lies in DENSITY_LIMIT_BAND of the
 * density limit for `density:limit`, the sustained seconds over the chart length for `density:sustained`, 1 for the chart's main
 * BPM band for `bpm:`, and 1 for a chart with soflan events for `soflan:`. The NPS, pair, band and soflan reads need current
 * features, so they are 0 for a chart without extras; lanes, hands, timing, fatigue and contexts do not pick charts and are 0.
 */
export const weaknessShareOf = (key: string, input: WeaknessShareInput) => {
    const parsed = parseWeaknessKey(key)
    if (!parsed) return 0
    const [tag] = parsed.tags
    if ((parsed.kind === 'tag' || parsed.kind === 'solo') && tag !== undefined) return input.tags?.[tag] ?? 0
    const extras = input.extras
    if (!extras) return 0
    if (parsed.kind === 'combo') return extras.combos[key.slice(key.indexOf(KEY_SEPARATOR) + 1)] ?? 0
    if (parsed.kind === 'bpm') return extras.bpmMainBand === parsed.arg ? 1 : 0
    if (parsed.kind === 'soflan') return extras.soflanEvents > 0 ? 1 : 0
    if (parsed.kind !== 'density') return 0
    if (parsed.arg === 'sustained')
        return extras.lengthMs > 0 ? roundRatio(Math.min(1, (extras.nps.sustained_s * MILLIS_PER_SECOND) / extras.lengthMs)) : 0
    if (input.limitNps === null) return 0
    const p90 = extras.nps.p90_2s
    return p90 >= input.limitNps * DENSITY_LIMIT_BAND.LOW && p90 <= input.limitNps * DENSITY_LIMIT_BAND.HIGH ? 1 : 0
}

/** Whether a chart's 90th-percentile NPS is beyond the EX density limit times DENSITY_OVER_RATIO; never without current features or a limit. */
export const isDensityOver = (extras: ChartFeatureExtras | undefined, limitNps: number | null) =>
    extras !== undefined && limitNps !== null && extras.nps.p90_2s > limitNps * DENSITY_OVER_RATIO

/**
 * The charts of a pool that train one weakness key, largest share first (then table order): a share of FOCUS_TAG_MIN_RATIO or
 * more, at most `limit`, each with its share.
 */
export const rankWeaknessCharts = (params: {
    pool: readonly ChartProgress[]
    key: string
    load: ChartTagLoad
    limitNps: number | null
    limit: number
}) =>
    params.pool
        .map((chart) => ({
            chart,
            share: weaknessShareOf(params.key, {
                tags: params.load.shares.get(chart.md5),
                extras: params.load.extras?.get(chart.md5),
                limitNps: params.limitNps,
            }),
        }))
        .filter((entry) => entry.share >= FOCUS_TAG_MIN_RATIO)
        .toSorted(
            (left, right) =>
                right.share - left.share || left.chart.levelIndex - right.chart.levelIndex || left.chart.md5.localeCompare(right.chart.md5),
        )
        .slice(0, params.limit)

/** The table, its charts with records, the level summary and the frontier: the start of every table-based feature. */
export const readTableProgress = (table: ProgressTable, records: ProgressRecords) => {
    const charts = toChartProgress(table, records)
    const levels = summarizeLevels(table, charts)
    const frontierIndex = findFrontierIndex(levels)
    return { charts, levels, frontierIndex, frontier: frontierIndex === null ? null : (levels[frontierIndex] ?? null) }
}

export type TableProgress = ReturnType<typeof readTableProgress>
