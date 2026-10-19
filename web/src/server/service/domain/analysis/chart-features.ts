import { CHART_PATTERN_NOTE_KIND, CHART_PATTERN_SCRATCH_LANE, type ChartPatternDocument } from '@server/dto/chart-pattern'
import { ANALYSIS_VERSION, RATIO_DECIMALS } from '@server/service/domain/analysis/analysis-constants'
import {
    LAYOUT_DEPENDENT_TAGS,
    PATTERN_NOTE_KIND,
    PATTERN_TAGS,
    classifyPattern,
    countSoflanEvents,
    densityWindowCounts,
    isJudgedNote,
    type PatternInput,
    type PatternNote,
    type PatternNoteKind,
    type PatternTag,
} from '@server/service/domain/analysis/pattern-tags'

const JUDGED_END_LN = new Set(['cn', 'hcn'])

/** Ratios and averages kept to RATIO_DECIMALS places so the stored document stays small and the same input gives the same bytes. */
export const roundRatio = (value: number) => {
    const scale = 10 ** RATIO_DECIMALS
    return Math.round(value * scale) / scale
}

const ratioOf = (part: number, whole: number) => (whole > 0 ? roundRatio(part / whole) : 0)

const MICROS_PER_MILLI = 1000
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

/**
 * The per-chart features stored with a pattern (plan 4.2, `chart_pattern.features`): how many judged notes carry each tag and their
 * share, density, soflan and LN shares. The lane-reading tags describe the chart's own layout with no option (`layout: "original"`);
 * a play under RANDOM-family options sees other lanes (plan 4.5).
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
    }
}

export type ChartFeatures = ReturnType<typeof computeChartFeatures>
