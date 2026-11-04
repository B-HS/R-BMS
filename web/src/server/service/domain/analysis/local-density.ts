import {
    BURST_RATIO,
    BURST_WINDOW_US,
    DENSITY_CONTEXT,
    LOCAL_NPS_WINDOW_US,
    NPS_BIN_TOP,
    NPS_BIN_WIDTH,
    PEAK_NPS_DECIMALS,
    PEAK_NPS_WINDOWS_MS,
    PEAK_RPS_WINDOW_MS,
    SUSTAIN_MIN_NPS,
    SUSTAIN_RATIO,
    SUSTAIN_WINDOW_US,
} from '@server/service/domain/analysis/analysis-constants'
import { isJudgedNote, type PatternNote } from '@server/service/domain/analysis/pattern-tags'
import { roundToDecimals } from '@server/service/domain/analysis/play-cells'

const MICROS_PER_SECOND = 1_000_000
const MICROS_PER_MILLI = 1000
const HALVES = 2

const densityContextOf = (nps1: number, nps2: number, nps8: number) => {
    if (nps1 >= BURST_RATIO * nps8) return DENSITY_CONTEXT.BURST
    if (nps2 >= SUSTAIN_MIN_NPS && nps8 >= SUSTAIN_RATIO * nps2) return DENSITY_CONTEXT.SUSTAINED
    return DENSITY_CONTEXT.MIXED
}

/** For each query time (ascending), how many of the ascending judged times lie in the window `[t - w/2, t + w/2)` centred on it. */
const centredCounts = (judgedTimes: readonly number[], queryTimes: readonly number[], windowUs: number) => {
    const half = windowUs / HALVES
    let low = 0
    let high = 0
    return queryTimes.map((timeUs) => {
        while (low < judgedTimes.length && judgedTimes[low] < timeUs - half) low += 1
        while (high < judgedTimes.length && judgedTimes[high] < timeUs + half) high += 1
        return high - low
    })
}

const judgedTimesOf = (notes: readonly PatternNote[]) =>
    notes
        .filter(isJudgedNote)
        .map((note) => note.timeUs)
        .toSorted((a, b) => a - b)

/** The length of a centred window cut to `[0, spanUs]`, never less than one microsecond past the note. */
const clippedWindowUs = (timeUs: number, windowUs: number, spanUs: number) => {
    const half = windowUs / HALVES
    return Math.max(Math.min(timeUs + half, spanUs), timeUs + 1) - Math.max(timeUs - half, 0)
}

/**
 * The local density of every note (plan 2026-11-04 2.2.1), one entry per input note in its order: the judged notes (reached or not)
 * per second in the windows of BURST_WINDOW_US, LOCAL_NPS_WINDOW_US and SUSTAIN_WINDOW_US centred on the note, each window cut to the
 * chart's notes (from time 0 to just past the last judged note) and divided by its cut length so a chart's start and end are not read
 * as sparse, and the note's density context. The notes alone set the span, so a chart and a play of it read the same densities: a
 * play's own length runs PLAY_TIME_MARGIN_MS (rbms-play) past its last note.
 */
export const localDensitiesOf = (notes: readonly PatternNote[]) => {
    const judgedTimes = judgedTimesOf(notes)
    const spanUs = (judgedTimes.at(-1) ?? 0) + 1
    const order = notes.map((_, index) => index).toSorted((a, b) => notes[a].timeUs - notes[b].timeUs)
    const queryTimes = order.map((index) => notes[index].timeUs)
    const npsOver = (windowUs: number) =>
        centredCounts(judgedTimes, queryTimes, windowUs).map(
            (count, at) => (count * MICROS_PER_SECOND) / clippedWindowUs(queryTimes[at], windowUs, spanUs),
        )
    const burst = npsOver(BURST_WINDOW_US)
    const local = npsOver(LOCAL_NPS_WINDOW_US)
    const sustained = npsOver(SUSTAIN_WINDOW_US)
    const rankOf = new Map(order.map((index, at) => [index, at]))
    return notes.map((_, index) => {
        const at = rankOf.get(index) ?? 0
        return { nps1: burst[at], nps2: local[at], nps8: sustained[at], context: densityContextOf(burst[at], local[at], sustained[at]) }
    })
}

export type LocalDensity = ReturnType<typeof localDensitiesOf>[number]

/** The NPS bin of a local NPS, by its lower edge: 0 to NPS_BIN_TOP. The top bin holds every NPS from NPS_BIN_TOP on. */
export const npsBinOf = (nps: number) => Math.min(Math.floor(nps / NPS_BIN_WIDTH) * NPS_BIN_WIDTH, NPS_BIN_TOP)

/** The stored key of an NPS bin: its lower edge, the top bin with a `+`. */
export const npsBinKeyOf = (bin: number) => (bin >= NPS_BIN_TOP ? `${NPS_BIN_TOP}+` : String(bin))

/** The most of the ascending times any window `[t, t + w)` starting at one of them holds, per second of the window. */
const peakRateOf = (sortedTimes: readonly number[], windowUs: number) => {
    let high = 0
    let peak = 0
    sortedTimes.forEach((timeUs, low) => {
        while (high < sortedTimes.length && sortedTimes[high] < timeUs + windowUs) high += 1
        peak = Math.max(peak, high - low)
    })
    return roundToDecimals((peak * MICROS_PER_SECOND) / windowUs, PEAK_NPS_DECIMALS)
}

/** The peak NPS of each of PEAK_NPS_WINDOWS_MS, keyed by the window in milliseconds. */
export const peakNpsOf = (notes: readonly PatternNote[]) => {
    const times = judgedTimesOf(notes)
    return Object.fromEntries(PEAK_NPS_WINDOWS_MS.map((windowMs) => [String(windowMs), peakRateOf(times, windowMs * MICROS_PER_MILLI)]))
}

/** The peak rows per second over PEAK_RPS_WINDOW_MS, a row being the judged notes of one time. */
export const peakRowsPerSecondOf = (notes: readonly PatternNote[]) =>
    peakRateOf([...new Set(judgedTimesOf(notes))], PEAK_RPS_WINDOW_MS * MICROS_PER_MILLI)
