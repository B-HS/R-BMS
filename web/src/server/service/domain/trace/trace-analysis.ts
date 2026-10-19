import { CHART_PATTERN_MODE, CHART_PATTERN_NOTE_KIND, type ChartPatternDocument } from '@server/dto/chart-pattern'
import { TRACE_NOTE_KIND, TRACE_NOTE_STATE, TRACE_SPEED_FIELDS, type TraceSpeedInput, type TraceUploadInput } from '@server/dto/trace'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { PATTERN_NOTE_KIND, type PatternNoteKind } from '@server/service/domain/analysis/pattern-tags'
import {
    computePlayMetrics,
    findGaugeDrops,
    readTraceGreenNumber,
    summarizeGaugeCurves,
    type GaugeCurve,
    type JudgedPlayNote,
} from '@server/service/domain/analysis/play-metrics'

/**
 * How a trace lines up with its chart's pattern (plan 3.4): `matched` when the pattern is of the same file (its sha256 is the
 * score's) and the trace's note times are the pattern's, `mismatch` when either differs, and `absent` when there is no pattern.
 * Only a matched trace has the chart's features joined to it.
 */
export const PATTERN_JOIN = { MATCHED: 'matched', MISMATCH: 'mismatch', ABSENT: 'absent' } as const
export type PatternJoin = (typeof PATTERN_JOIN)[keyof typeof PATTERN_JOIN]

/** How the run ended, read off the trace: played to its end, ended by an empty gauge, or left before its end. */
export const RUN_END = { FINISHED: 'finished', GAUGE_EMPTY: 'gauge_empty', LEFT: 'left' } as const

/** The pattern analysis classifies 7-key single-play layouts only (plan 4.1); a trace of another mode keeps its gauge and speed figures. */
const ANALYZED_MODE = CHART_PATTERN_MODE

const MICROS_PER_MILLI = 1000

const KIND_OF_CODE: Partial<Record<number, PatternNoteKind>> = {
    [TRACE_NOTE_KIND.NORMAL]: PATTERN_NOTE_KIND.NORMAL,
    [TRACE_NOTE_KIND.LONG_HEAD]: PATTERN_NOTE_KIND.LN_HEAD,
    [TRACE_NOTE_KIND.LONG_END]: PATTERN_NOTE_KIND.LN_END,
    [TRACE_NOTE_KIND.CHARGE_END]: PATTERN_NOTE_KIND.CHARGE_END,
}

const byTimeThenLane = (a: { timeUs: number; lane: number }, b: { timeUs: number; lane: number }) => a.timeUs - b.timeUs || a.lane - b.lane

const playNotesOf = (trace: TraceUploadInput) =>
    trace.notes.time_us
        .map(
            (timeUs, index) =>
                ({
                    timeUs,
                    lane: trace.notes.lane[index],
                    kind: KIND_OF_CODE[trace.notes.kind[index]] ?? PATTERN_NOTE_KIND.LN_END,
                    state: trace.notes.state[index],
                    errUs: trace.notes.err_us[index],
                }) satisfies JudgedPlayNote,
        )
        .toSorted(byTimeThenLane)

const tempoOf = (trace: TraceUploadInput) =>
    trace.tempo.map((point) => ({ tUs: point.t_us, bpm: point.bpm, stopUs: point.stop_us, scroll: point.scroll })).toSorted((a, b) => a.tUs - b.tUs)

const isSameMultiset = (a: readonly number[], b: readonly number[]) => {
    if (a.length !== b.length) return false
    const sortedA = a.toSorted((x, y) => x - y)
    const sortedB = b.toSorted((x, y) => x - y)
    return sortedA.every((value, index) => value === sortedB[index])
}

/**
 * Whether the trace is a run of the pattern's chart (plan 3.4). Lane options never move a note in time, so the times of every
 * object the trace holds (normal notes, LN heads and both kinds of LN end) are the times of the pattern's notes but its mines,
 * leaving out the lanes SCRATCH AUTO played (`auto_lanes`, which the trace leaves out too). Anything else is a mismatch.
 */
export const joinPattern = (params: { trace: TraceUploadInput; chartSha256: string; pattern: ChartPatternDocument | null }) => {
    const { trace, pattern } = params
    if (!pattern) return PATTERN_JOIN.ABSENT
    if (pattern.sha256 !== params.chartSha256) return PATTERN_JOIN.MISMATCH
    const patternTimes = pattern.notes.time_us.filter(
        (_, index) => pattern.notes.kind[index] !== CHART_PATTERN_NOTE_KIND.MINE && trace.auto_lanes[pattern.notes.lane[index]] !== true,
    )
    return isSameMultiset(patternTimes, trace.notes.time_us) ? PATTERN_JOIN.MATCHED : PATTERN_JOIN.MISMATCH
}

const runEndOf = (trace: TraceUploadInput) => {
    if (trace.failed_at_ms !== null) return RUN_END.GAUGE_EMPTY
    const isUnreached = (state: number, index: number) => state === TRACE_NOTE_STATE.NONE && trace.notes.kind[index] !== TRACE_NOTE_KIND.LONG_END
    return trace.notes.state.some(isUnreached) ? RUN_END.LEFT : RUN_END.FINISHED
}

const gaugeCurvesOf = (trace: TraceUploadInput) =>
    trace.gauge.series.flatMap((values, index) => {
        const bounds = trace.gauge_bounds.at(index)
        if (!bounds) return []
        const failedAtMs = index === trace.gauge.selected ? trace.failed_at_ms : null
        return [
            { intervalMs: trace.gauge.interval_ms, values, min: bounds.min, max: bounds.max, border: bounds.border, failedAtMs } satisfies GaugeCurve,
        ]
    })

const greenOf = (speed: TraceSpeedInput) => readTraceGreenNumber({ green: speed.green, greenMs: speed.green_ms })

/**
 * The stored analysis of one run (`play_analysis.summary`, plan 4.3) from its trace alone and, when there is one, its chart's
 * pattern: the per-play figures on the lanes the player saw (7-key single play only), how the run ended, every gauge's end and the
 * played gauge's drops, the green number it started and ended on with the number of setting changes, and the pattern join.
 */
export const analyzeTrace = (params: { trace: TraceUploadInput; chartSha256: string; pattern: ChartPatternDocument | null }) => {
    const { trace } = params
    const patternJoin = joinPattern(params)
    const curves = gaugeCurvesOf(trace)
    const played = curves.at(trace.gauge.selected)
    const tempo = tempoOf(trace)
    const isAnalyzedMode = trace.mode === ANALYZED_MODE
    const metrics = isAnalyzedMode
        ? computePlayMetrics({
              notes: playNotesOf(trace),
              tempo,
              initBpm: tempo.at(0)?.bpm ?? 0,
              emptyPoorTimesUs: trace.empty_poors.time_us,
              lengthUs: trace.play_time_ms * MICROS_PER_MILLI,
          })
        : null
    const patternMd5 = patternJoin === PATTERN_JOIN.MATCHED ? (params.pattern?.md5 ?? null) : null
    const summary = {
        analysis_version: ANALYSIS_VERSION,
        format_version: trace.version,
        mode: trace.mode,
        random: trace.random,
        lntype: trace.lntype,
        scratch_auto: trace.auto_lanes.some(Boolean),
        run_end: runEndOf(trace),
        metrics,
        gauge: {
            selected: trace.gauge.selected,
            interval_ms: trace.gauge.interval_ms,
            failed_at_ms: trace.failed_at_ms,
            drops: played ? findGaugeDrops(played) : [],
            curves: summarizeGaugeCurves(curves),
        },
        speed: {
            start: greenOf(trace.speed.start),
            end: greenOf(trace.speed.end),
            changes: trace.speed.changes.length,
            changed_fields: TRACE_SPEED_FIELDS.filter((field) => trace.speed.changes.some((change) => change.field === field)),
        },
        pattern: { join: patternJoin, md5: patternMd5 },
    }
    return { summary, patternJoin, patternMd5 }
}

export type TraceAnalysis = ReturnType<typeof analyzeTrace>
export type TraceAnalysisSummary = TraceAnalysis['summary']
