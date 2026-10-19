import { CHART_PATTERN_MODE, CHART_PATTERN_NOTE_KIND, type ChartPatternDocument } from '@server/dto/chart-pattern'
import { TRACE_NOTE_KIND, TRACE_NOTE_STATE, TRACE_SPEED_FIELDS, type TraceSpeedInput, type TraceUploadInput } from '@server/dto/trace'
import { ANALYSIS_VERSION, PLAY_ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { PATTERN_NOTE_KIND, type PatternNoteKind } from '@server/service/domain/analysis/pattern-tags'
import {
    RUN_END,
    computePlayMetrics,
    findGaugeDrops,
    liveGaugeValues,
    readTraceGreenNumber,
    summarizeGaugeCurves,
    type GaugeCurve,
    type GaugeRun,
    type JudgedPlayNote,
} from '@server/service/domain/analysis/play-metrics'
import type { RandomOption } from '@server/dto/common'

/**
 * How a trace lines up with its chart's pattern (plan 3.4): `matched` when the pattern is of the same file (its sha256 is the
 * score's) and the trace's note times are the pattern's, `mismatch` when either differs, `unsupported` when the run's notes cannot
 * be told from the pattern's (a lane option that moves notes onto the scratch lane with SCRATCH AUTO playing it, while the trace's
 * times are among the pattern's), and `absent` when there is no pattern. Only a matched trace has the chart's features joined to it.
 */
export const PATTERN_JOIN = { MATCHED: 'matched', MISMATCH: 'mismatch', UNSUPPORTED: 'unsupported', ABSENT: 'absent' } as const
export type PatternJoin = (typeof PATTERN_JOIN)[keyof typeof PATTERN_JOIN]

/**
 * Lane options that move notes between the keys and the scratch lane (rbms-chart `shuffle.rs`: ALL-SCRATCH fills the scratch first;
 * every other option keeps the scratch lane fixed). Under SCRATCH AUTO the notes such an option put on the scratch were played by
 * the game and are not in the trace, and which original notes they were is not in it either.
 */
const SCRATCH_MOVING_OPTIONS: ReadonlySet<RandomOption> = new Set(['AllScratch'])

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

const countTimes = (times: readonly number[]) => {
    const counts = new Map<number, number>()
    for (const time of times) counts.set(time, (counts.get(time) ?? 0) + 1)
    return counts
}

const isSameMultiset = (a: readonly number[], b: readonly number[]) => {
    if (a.length !== b.length) return false
    const sortedA = a.toSorted((x, y) => x - y)
    const sortedB = b.toSorted((x, y) => x - y)
    return sortedA.every((value, index) => value === sortedB[index])
}

const isSubMultiset = (part: readonly number[], whole: readonly number[]) => {
    const available = countTimes(whole)
    return [...countTimes(part)].every(([time, count]) => (available.get(time) ?? 0) >= count)
}

/**
 * Whether the trace is a run of the pattern's chart (plan 3.4). Lane options never move a note in time, so the times of every
 * object the trace holds (normal notes, LN heads and both kinds of LN end) are the times of the pattern's notes but its mines,
 * leaving out the lanes SCRATCH AUTO played (`auto_lanes`, which the trace leaves out too). When a lane option moved notes onto those
 * lanes (SCRATCH_MOVING_OPTIONS), the notes left out are the played layout's, not the original's: the trace's times are then only
 * checked to be among the pattern's, and the join is `unsupported` rather than `matched`. Anything else is a mismatch.
 */
export const joinPattern = (params: { trace: TraceUploadInput; chartSha256: string; pattern: ChartPatternDocument | null }) => {
    const { trace, pattern } = params
    if (!pattern) return PATTERN_JOIN.ABSENT
    if (pattern.sha256 !== params.chartSha256) return PATTERN_JOIN.MISMATCH
    const objectTimes = pattern.notes.time_us.filter((_, index) => pattern.notes.kind[index] !== CHART_PATTERN_NOTE_KIND.MINE)
    if (trace.auto_lanes.some(Boolean) && SCRATCH_MOVING_OPTIONS.has(trace.random)) {
        return isSubMultiset(trace.notes.time_us, objectTimes) ? PATTERN_JOIN.UNSUPPORTED : PATTERN_JOIN.MISMATCH
    }
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
        return [{ intervalMs: trace.gauge.interval_ms, values, min: bounds.min, max: bounds.max, border: bounds.border } satisfies GaugeCurve]
    })

/**
 * The latest instant of a run its gauge log covers: an empty gauge's failure, or the last sample of a run that was left (the log
 * stops when the player leaves). A run played to its end is covered throughout.
 */
const lastKnownMsOf = (curve: GaugeCurve, run: Pick<GaugeRun, 'end' | 'failedAtMs'>) => {
    if (run.end === RUN_END.GAUGE_EMPTY) return run.failedAtMs ?? Number.POSITIVE_INFINITY
    if (run.end === RUN_END.LEFT) return (curve.values.length - 1) * curve.intervalMs
    return Number.POSITIVE_INFINITY
}

/**
 * The played gauge at the end of each segment, from the samples the run logged. A run played to its end may log its last sample a
 * little before the end of the last segment, which then reads that sample; after an empty gauge's failure or after the player left
 * (Escape, quit) nothing is known (null), so a segment the run never reached does not read the last sample.
 */
const segmentGaugeValues = (curve: GaugeCurve | undefined, segmentEndsMs: readonly number[], run: Pick<GaugeRun, 'end' | 'failedAtMs'>) =>
    segmentEndsMs.map((endMs) => {
        if (!curve || endMs > lastKnownMsOf(curve, run)) return null
        const index = Math.min(Math.floor(endMs / curve.intervalMs), curve.values.length - 1)
        return curve.values.at(Math.max(0, index)) ?? null
    })

const greenOf = (speed: TraceSpeedInput) => readTraceGreenNumber({ green: speed.green, greenMs: speed.green_ms })

/**
 * The stored analysis of one run (`play_analysis.summary`, plan 4.3) from its trace alone and, when there is one, its chart's
 * pattern: the per-play figures on the lanes the player saw (7-key single play only), how the run ended, every gauge's end (read only
 * up to an empty gauge's failure) with the played gauge's drops and its value at each segment's end, the green number it started and
 * ended on with the number of setting changes, and the pattern join.
 */
export const analyzeTrace = (params: { trace: TraceUploadInput; chartSha256: string; pattern: ChartPatternDocument | null }) => {
    const { trace } = params
    const patternJoin = joinPattern(params)
    const runEnd = runEndOf(trace)
    const run = { end: runEnd, failedAtMs: trace.failed_at_ms, selected: trace.gauge.selected }
    const curves = gaugeCurvesOf(trace)
    const playedCurve = curves.at(trace.gauge.selected)
    const played = playedCurve ? { ...playedCurve, values: liveGaugeValues(playedCurve, run) } : undefined
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
        analysis_version: PLAY_ANALYSIS_VERSION,
        pattern_analysis_version: ANALYSIS_VERSION,
        format_version: trace.version,
        mode: trace.mode,
        random: trace.random,
        lntype: trace.lntype,
        scratch_auto: trace.auto_lanes.some(Boolean),
        run_end: runEnd,
        metrics,
        gauge: {
            selected: trace.gauge.selected,
            interval_ms: trace.gauge.interval_ms,
            failed_at_ms: trace.failed_at_ms,
            drops: played ? findGaugeDrops(played) : [],
            curves: summarizeGaugeCurves(curves, run),
            segment_values: metrics
                ? segmentGaugeValues(
                      played,
                      metrics.segments.map((segment) => segment.to_ms),
                      run,
                  )
                : [],
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

/** The two generations a stored analysis carries: the play summary's (PLAY_ANALYSIS_VERSION) and the pattern classification's (ANALYSIS_VERSION). */
export type AnalysisVersions = { play: number; pattern: number }

export const CURRENT_ANALYSIS_VERSIONS = { play: PLAY_ANALYSIS_VERSION, pattern: ANALYSIS_VERSION } satisfies AnalysisVersions

/**
 * What `play_analysis.summary` holds for a trace whose analysis failed at the current versions (`pattern_join` is `failed`): it is
 * not read as a summary and not computed again until one of the two versions goes up.
 */
export const analysisFailureSummary = () => ({
    analysis_version: PLAY_ANALYSIS_VERSION,
    pattern_analysis_version: ANALYSIS_VERSION,
    failed: true as const,
})

export type TraceAnalysisFailure = ReturnType<typeof analysisFailureSummary>

/** The `play_analysis.pattern_join` of a failure marker: not a join result, so it is none of PATTERN_JOIN. */
export const ANALYSIS_FAILED_JOIN = 'failed'
