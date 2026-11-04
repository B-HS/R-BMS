import { z } from 'zod'
import { CHART_PATTERN_NOTE_KIND, CHART_PATTERN_SCRATCH_LANE, type ChartPatternDocument } from '@server/dto/chart-pattern'
import {
    ANALYSIS_VERSION,
    BPM_BANDS,
    CHART_COMBO_TOP,
    CHART_NPS_PERCENTILES,
    DENSITY_CONTEXT,
    NPS_SUMMARY_DECIMALS,
    PEAK_NPS_DECIMALS,
    RATIO_DECIMALS,
    SUSTAIN_GAP_MAX_US,
    type BpmBand,
} from '@server/service/domain/analysis/analysis-constants'
import { localDensitiesOf } from '@server/service/domain/analysis/local-density'
import {
    LAYOUT_DEPENDENT_TAGS,
    PATTERN_NOTE_KIND,
    PATTERN_TAGS,
    classifyPattern,
    countSoflanEvents,
    createTempoLookup,
    densityWindowCounts,
    isJudgedNote,
    type PatternInput,
    type PatternNote,
    type PatternNoteKind,
    type PatternTag,
} from '@server/service/domain/analysis/pattern-tags'
import { bpmBandOf, roundToDecimals } from '@server/service/domain/analysis/play-cells'
import { COMBO_KEYS, comboKeysOf, comboTagsOf } from '@server/service/domain/analysis/tag-combos'

const JUDGED_END_LN = new Set(['cn', 'hcn'])

/** Ratios and averages kept to RATIO_DECIMALS places so the stored document stays small and the same input gives the same bytes. */
export const roundRatio = (value: number) => {
    const scale = 10 ** RATIO_DECIMALS
    return Math.round(value * scale) / scale
}

const ratioOf = (part: number, whole: number) => (whole > 0 ? roundRatio(part / whole) : 0)

const MICROS_PER_MILLI = 1000
const MICROS_PER_SECOND = 1_000_000
const PERCENT = 100
/** The late part of a chart, for the late density average: its last third of seconds. */
const LATE_PART_DIVISOR = 3
/** `note_density` (rbms-chart) averages only the seconds holding at least a quarter of the mean count. */
const DENSITY_AVERAGE_FLOOR_DIVISOR = 4

/**
 * Whether the chart judges its LN ends: a CN or HCN chart does. A chart with no stated flavour (`declared_ln` null) is read as LN,
 * the default, so its ends are not judged.
 */
const judgesLongEnds = (document: ChartPatternDocument) => document.declared_ln !== null && JUDGED_END_LN.has(document.declared_ln)

const KIND_OF_CODE: Partial<Record<number, PatternNoteKind>> = {
    [CHART_PATTERN_NOTE_KIND.NORMAL]: PATTERN_NOTE_KIND.NORMAL,
    [CHART_PATTERN_NOTE_KIND.LONG_HEAD]: PATTERN_NOTE_KIND.LN_HEAD,
    [CHART_PATTERN_NOTE_KIND.LONG_END]: PATTERN_NOTE_KIND.LN_END,
    [CHART_PATTERN_NOTE_KIND.MINE]: PATTERN_NOTE_KIND.MINE,
}

/** The pattern document as the classifier's input: the chart's own layout, LN ends judged when the chart is CN or HCN. */
export const toPatternInput = (document: ChartPatternDocument) => {
    const endKind = judgesLongEnds(document) ? PATTERN_NOTE_KIND.CHARGE_END : PATTERN_NOTE_KIND.LN_END
    const notes = document.notes.time_us.map((timeUs, index) => {
        const kind = KIND_OF_CODE[document.notes.kind[index]] ?? PATTERN_NOTE_KIND.MINE
        return { timeUs, lane: document.notes.lane[index], kind: kind === PATTERN_NOTE_KIND.LN_END ? endKind : kind } satisfies PatternNote
    })
    const tempo = document.tempo.map((point) => ({ tUs: point.t_us, bpm: point.bpm, stopUs: point.stop_us, scroll: point.scroll }))
    return { notes, tempo, initBpm: document.init_bpm }
}

const densityOf = (notes: readonly PatternNote[]) => {
    const counts = densityWindowCounts(notes)
    if (counts.length === 0) return { peak: 0, average: 0, late_average: 0 }
    const total = counts.reduce((sum, count) => sum + count, 0)
    const floor = Math.floor(total / counts.length / DENSITY_AVERAGE_FLOOR_DIVISOR)
    const counted = counts.filter((count) => count >= floor)
    const late = counts.slice(Math.floor((counts.length * (LATE_PART_DIVISOR - 1)) / LATE_PART_DIVISOR))
    return {
        peak: counts.reduce((peak, count) => Math.max(peak, count), 0),
        average: ratioOf(
            counted.reduce((sum, count) => sum + count, 0),
            counted.length,
        ),
        late_average: ratioOf(
            late.reduce((sum, count) => sum + count, 0),
            late.length,
        ),
    }
}

const bpmRangeOf = (input: PatternInput) => {
    const positive = [input.initBpm, ...input.tempo.map((point) => point.bpm)].filter((bpm) => bpm > 0)
    if (positive.length === 0) return { min: 0, max: 0, max_to_min: 0 }
    const min = positive.reduce((low, bpm) => Math.min(low, bpm), Infinity)
    const max = positive.reduce((high, bpm) => Math.max(high, bpm), 0)
    return { min: roundRatio(min), max: roundRatio(max), max_to_min: roundRatio(max / min) }
}

const chartNpsSchema = z.object({ p50_2s: z.number(), p90_2s: z.number(), max_2s: z.number(), max_8s: z.number(), sustained_s: z.number() })
export type ChartNps = z.infer<typeof chartNpsSchema>

/** The chart features ANALYSIS_VERSION 2 added, the only ones read through `readChartFeatureExtras`. */
const chartFeatureExtrasSchema = z.object({ nps: chartNpsSchema, combos: z.record(z.string(), z.number()), bpm_main_band: z.enum(BPM_BANDS) })

/** The nearest-rank percentile of ascending values, the smallest value at least `percentile`% of them do not exceed. */
const nearestRank = (sorted: readonly number[], percentile: number) => sorted[Math.max(0, Math.ceil((percentile / PERCENT) * sorted.length) - 1)] ?? 0

/**
 * The sliding-window NPS of a chart's judged notes (plan 2026-11-04 2.3, `localDensitiesOf`): the nearest-rank median and 90th
 * percentile and the most of the notes' LOCAL_NPS_WINDOW_US NPS, the most of their SUSTAIN_WINDOW_US NPS, and the seconds sustained
 * notes follow one another at most SUSTAIN_GAP_MAX_US apart.
 */
const chartNpsOf = (notes: readonly PatternNote[]) => {
    const densities = localDensitiesOf(notes)
    const judged = notes.flatMap((note, index) => (isJudgedNote(note) ? [{ timeUs: note.timeUs, density: densities[index] }] : []))
    const local = judged.map(({ density }) => density.nps2).toSorted((a, b) => a - b)
    const sustainedTimes = judged
        .filter(({ density }) => density.context === DENSITY_CONTEXT.SUSTAINED)
        .map(({ timeUs }) => timeUs)
        .toSorted((a, b) => a - b)
    const sustainedUs = sustainedTimes.reduce((sum, timeUs, index) => {
        const gap = index > 0 ? timeUs - sustainedTimes[index - 1] : 0
        return gap <= SUSTAIN_GAP_MAX_US ? sum + gap : sum
    }, 0)
    const roundNps = (value: number) => roundToDecimals(value, PEAK_NPS_DECIMALS)
    return {
        p50_2s: roundNps(nearestRank(local, CHART_NPS_PERCENTILES.MEDIAN)),
        p90_2s: roundNps(nearestRank(local, CHART_NPS_PERCENTILES.HIGH)),
        max_2s: roundNps(local.at(-1) ?? 0),
        max_8s: roundNps(judged.reduce((peak, { density }) => Math.max(peak, density.nps8), 0)),
        sustained_s: roundToDecimals(sustainedUs / MICROS_PER_SECOND, NPS_SUMMARY_DECIMALS),
    } satisfies ChartNps
}

/** The shares of judged notes carrying each tag pair (the pairs of `by_combo`), the CHART_COMBO_TOP largest, COMBO_KEYS order between equals. */
const chartCombosOf = (tagLists: readonly (readonly PatternTag[])[], judgedIndexes: readonly number[]) => {
    const counts = new Map<string, number>()
    for (const index of judgedIndexes) {
        for (const key of comboKeysOf(comboTagsOf(tagLists[index]))) counts.set(key, (counts.get(key) ?? 0) + 1)
    }
    return Object.fromEntries(
        COMBO_KEYS.flatMap((key) => (counts.has(key) ? [[key, counts.get(key) ?? 0] as const] : []))
            .toSorted(([, left], [, right]) => right - left)
            .slice(0, CHART_COMBO_TOP)
            .map(([key, notes]) => [key, ratioOf(notes, judgedIndexes.length)]),
    )
}

/** The BPM band most judged notes are played in, the lowest band between equals; a chart with none is in the band of its opening tempo. */
const bpmMainBandOf = (input: PatternInput, judgedIndexes: readonly number[]) => {
    const lookup = createTempoLookup(input.tempo, input.initBpm)
    if (judgedIndexes.length === 0) return bpmBandOf(lookup.bpmAt(0))
    const counts = new Map<BpmBand, number>()
    for (const index of judgedIndexes) {
        const band = bpmBandOf(lookup.bpmAt(input.notes[index].timeUs))
        counts.set(band, (counts.get(band) ?? 0) + 1)
    }
    return BPM_BANDS.reduce((main, band) => ((counts.get(band) ?? 0) > (counts.get(main) ?? 0) ? band : main), BPM_BANDS[0])
}

/**
 * The per-chart features stored with a pattern (plan 4.2, `chart_pattern.features`): how many judged notes carry each tag and their
 * share, density, soflan and LN shares, and (ANALYSIS_VERSION 2) the sliding-window NPS, the largest tag-pair shares and the main BPM
 * band. The lane-reading tags describe the chart's own layout with no option (`layout: "original"`); a play under RANDOM-family
 * options sees other lanes (plan 4.5).
 */
export const computeChartFeatures = (document: ChartPatternDocument) => {
    const input = toPatternInput(document)
    const tagLists = classifyPattern(input)
    const judgedIndexes = input.notes.flatMap((note, index) => (isJudgedNote(note) ? [index] : []))
    const judged = judgedIndexes.length
    const tagCounts = new Map<PatternTag, number>()
    for (const index of judgedIndexes) {
        for (const tag of tagLists[index]) tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1)
    }
    const hits = input.notes.filter((note) => note.kind === PATTERN_NOTE_KIND.NORMAL || note.kind === PATTERN_NOTE_KIND.LN_HEAD)
    const lastTimeUs = input.notes.reduce((latest, note) => Math.max(latest, note.timeUs), 0)
    const soflan = countSoflanEvents(input.tempo, input.initBpm)

    return {
        analysis_version: ANALYSIS_VERSION,
        layout: 'original' as const,
        layout_dependent_tags: LAYOUT_DEPENDENT_TAGS,
        judged_notes: judged,
        ln_end_judged: judgesLongEnds(document),
        length_ms: Math.round(lastTimeUs / MICROS_PER_MILLI),
        tags: Object.fromEntries(
            PATTERN_TAGS.map((tag) => {
                const notes = tagCounts.get(tag) ?? 0
                return [tag, { notes, ratio: ratioOf(notes, judged) }]
            }),
        ),
        density: densityOf(input.notes),
        soflan: { tempo_jumps: soflan.jumps, long_stops: soflan.stops, bpm: bpmRangeOf(input) },
        ln_ratio: ratioOf(hits.filter((note) => note.kind === PATTERN_NOTE_KIND.LN_HEAD).length, hits.length),
        scratch_ratio: ratioOf(hits.filter((note) => note.lane === CHART_PATTERN_SCRATCH_LANE).length, hits.length),
        mines: input.notes.filter((note) => note.kind === PATTERN_NOTE_KIND.MINE).length,
        nps: chartNpsOf(input.notes),
        combos: chartCombosOf(tagLists, judgedIndexes),
        bpm_main_band: bpmMainBandOf(input, judgedIndexes),
    }
}

export type ChartFeatures = ReturnType<typeof computeChartFeatures>

/**
 * The features ANALYSIS_VERSION 2 added, read from stored features of any version: null when they are not there or not of their
 * shape, as with features of version 1 kept until the chart is computed again (plan 2026-11-04 2.3). Readers of `nps`, `combos` and
 * `bpm_main_band` go through this rather than the stored document.
 */
export const readChartFeatureExtras = (features: unknown) => {
    const parsed = chartFeatureExtrasSchema.safeParse(features)
    if (!parsed.success) return null
    return { nps: parsed.data.nps, combos: parsed.data.combos, bpmMainBand: parsed.data.bpm_main_band }
}
