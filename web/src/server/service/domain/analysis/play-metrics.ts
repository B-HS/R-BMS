import {
    ANALYSIS_VERSION,
    BPM_BANDS,
    CONTEXT_TOP_TAGS,
    DENSITY_BUCKET_TOP,
    DENSITY_BUCKET_WIDTH,
    DENSITY_CONTEXT,
    GAUGE_DROP_LOOKBACK_MS,
    GAUGE_DROP_MIN_RATIO,
    GAUGE_DROP_WINDOW_MS,
    GREEN_DISPLAY_FACTOR,
    HAND_KEYS,
    NPS_SUMMARY_DECIMALS,
    SEGMENT_COUNT,
    SEGMENT_TOP_TAGS,
    SOFLAN_PHASE_MS,
    SOFLAN_SETTLED_MS,
    SP7_LANE_HAND_KEYS,
    SP7_LANE_NAMES,
    TIMING_HISTOGRAM_RANGE_MS,
} from '@server/service/domain/analysis/analysis-constants'
import { computeBpRuns } from '@server/service/domain/analysis/bp-runs'
import { roundRatio } from '@server/service/domain/analysis/chart-features'
import { localDensitiesOf, npsBinKeyOf, npsBinOf, peakNpsOf, peakRowsPerSecondOf } from '@server/service/domain/analysis/local-density'
import {
    PATTERN_TAGS,
    classifyPattern,
    createTempoLookup,
    densityWindowCounts,
    densityWindowOf,
    isJudgedNote,
    soflanEventsOf,
    type PatternInput,
    type PatternNote,
    type PatternTag,
} from '@server/service/domain/analysis/pattern-tags'
import {
    EX_OF_GRADE,
    EX_PER_GREAT,
    EX_PER_PGREAT,
    GRADES,
    addToCell,
    addToMoments,
    bpmBandOf,
    cellRecordOf,
    cellsByKey,
    countTagsOf,
    emptyCompactCell,
    emptyTimingMoments,
    finishMoments,
    momentsRecordOf,
    roundToDecimals,
    tagCountRecordOf,
    topTagsOf,
    type CompactCell,
    type Grade,
} from '@server/service/domain/analysis/play-cells'
import { computeTagCombos } from '@server/service/domain/analysis/tag-combos'

/** `NoteMark.state` of the judge engine: 0 not judged (not reached, or an LN end that is never judged), 1 PGREAT to 5 missed POOR. */
export const JUDGE_STATE = { NONE: 0, PGREAT: 1, GREAT: 2, GOOD: 3, BAD: 4, POOR: 5 } as const

/** A note of a play as the judge engine left it, on the lane the player saw. `errUs` is positive when early. */
export type JudgedPlayNote = PatternNote & { state: number; errUs: number }

/** A section of a training chart (plan 2026-11-04 2.2.8): its place in the chart's spec, its kind, its focus and its chart time span. */
export type TrainingSectionSpan = { index: number; kind: string; focus_key: string; from_us: number; to_us: number }

/** A drop of the played gauge (`findGaugeDrops`), by its span only. */
export type GaugeDropSpan = { from_ms: number; to_ms: number }

export type PlayMetricsInput = Omit<PatternInput, 'notes'> & {
    notes: readonly JudgedPlayNote[]
    emptyPoorTimesUs: readonly number[]
    lengthUs: number
    gaugeDropsMs: readonly GaugeDropSpan[]
    trainingSections?: readonly TrainingSectionSpan[] | null
}

type TimedGrade = Exclude<Grade, 'poor'>

const GRADE_OF_STATE: Partial<Record<number, Grade>> = {
    [JUDGE_STATE.PGREAT]: 'pgreat',
    [JUDGE_STATE.GREAT]: 'great',
    [JUDGE_STATE.GOOD]: 'good',
    [JUDGE_STATE.BAD]: 'bad',
    [JUDGE_STATE.POOR]: 'poor',
}

const MICROS_PER_MILLI = 1000
const PLAY_THIRDS = 3

type TallyCounts = Record<Grade, number> & { fast: Record<TimedGrade, number>; slow: Record<TimedGrade, number> }

const emptyTally = () =>
    ({
        pgreat: 0,
        great: 0,
        good: 0,
        bad: 0,
        poor: 0,
        fast: { pgreat: 0, great: 0, good: 0, bad: 0 },
        slow: { pgreat: 0, great: 0, good: 0, bad: 0 },
    }) satisfies TallyCounts

const addNote = (tally: TallyCounts, grade: Grade, errUs: number) => {
    tally[grade] += 1
    if (grade === 'poor') return
    const side = errUs >= 0 ? tally.fast : tally.slow
    side[grade] += 1
}

/** EX ratio is `(2 PGREAT + GREAT) / (2 N)` over the notes that took a judgement, and null when no note did (no data, not a ratio of 0). */
export const finishTally = (tally: TallyCounts) => {
    const notes = GRADES.reduce((sum, grade) => sum + tally[grade], 0)
    const exScore = EX_PER_PGREAT * tally.pgreat + EX_PER_GREAT * tally.great
    return { ...tally, notes, ex_score: exScore, ex_ratio: notes > 0 ? roundRatio(exScore / (EX_PER_PGREAT * notes)) : null }
}

export type Tally = ReturnType<typeof finishTally>

const quantile = (sorted: readonly number[], share: number) => {
    if (sorted.length === 0) return 0
    const position = (sorted.length - 1) * share
    const lower = Math.floor(position)
    const upper = Math.ceil(position)
    return sorted[lower] + (sorted[upper] - sorted[lower]) * (position - lower)
}

const QUARTILE = { FIRST: 0.25, MEDIAN: 0.5, THIRD: 0.75 } as const

/** The timing of PGREAT and GREAT notes in microseconds, and a histogram of whole milliseconds (truncated toward zero, as `TimingDistribution`). */
export const summarizeTiming = (errorsUs: readonly number[]) => {
    const sorted = errorsUs.toSorted((a, b) => a - b)
    const count = sorted.length
    const mean = count > 0 ? sorted.reduce((sum, value) => sum + value, 0) / count : 0
    const variance = count > 0 ? sorted.reduce((sum, value) => sum + (value - mean) ** 2, 0) / count : 0
    const buckets = new Map<number, number>()
    for (const errorUs of sorted) {
        const ms = Math.trunc(errorUs / MICROS_PER_MILLI)
        if (Math.abs(ms) <= TIMING_HISTOGRAM_RANGE_MS) buckets.set(ms, (buckets.get(ms) ?? 0) + 1)
    }
    return {
        notes: count,
        mean_us: Math.round(mean),
        median_us: Math.round(quantile(sorted, QUARTILE.MEDIAN)),
        stdev_us: Math.round(Math.sqrt(variance)),
        iqr_us: Math.round(quantile(sorted, QUARTILE.THIRD) - quantile(sorted, QUARTILE.FIRST)),
        histogram_ms: [...buckets.entries()].toSorted(([a], [b]) => a - b).map(([ms, notes]) => ({ ms, notes })),
    }
}

const densityBucketOf = (count: number) => {
    if (count >= DENSITY_BUCKET_TOP) return `${DENSITY_BUCKET_TOP}+`
    const low = Math.floor(count / DENSITY_BUCKET_WIDTH) * DENSITY_BUCKET_WIDTH
    return `${low}-${low + DENSITY_BUCKET_WIDTH - 1}`
}

const segmentId = (index: number) => `s${String(index).padStart(2, '0')}`

const tallyGroups = <Key extends string>(entries: Iterable<[Key, Grade, number]>) => {
    const groups = new Map<Key, TallyCounts>()
    for (const [key, grade, errUs] of entries) {
        const tally = groups.get(key) ?? emptyTally()
        groups.set(key, tally)
        addNote(tally, grade, errUs)
    }
    return Object.fromEntries([...groups.entries()].map(([key, tally]) => [key, finishTally(tally)]))
}

const laneNameOf = (lane: number) => SP7_LANE_NAMES[lane] ?? String(lane)

/** A judged object of the chart (reached or not) as the context figures read it; `grade` is null when it took no judgement. */
type ContextNote = { timeUs: number; tags: readonly PatternTag[]; nps2: number; grade: Grade | null }

const isBreakGrade = (grade: Grade | null) => grade === 'bad' || grade === 'poor'

/**
 * What the chart held around each drop of the played gauge (plan 2026-11-04 2.2.5): over the drop's span from GAUGE_DROP_LOOKBACK_MS
 * before it starts, the judged objects, BAD plus POOR, their mean local NPS and their most frequent tags; and over every drop's span
 * together, each note that took a judgement counted once, the tag counts and the number of notes.
 */
const gaugeDropFiguresOf = (drops: readonly GaugeDropSpan[], notes: readonly ContextNote[]) => {
    const spans = drops.map((drop) => ({
        drop,
        fromUs: (drop.from_ms - GAUGE_DROP_LOOKBACK_MS) * MICROS_PER_MILLI,
        toUs: drop.to_ms * MICROS_PER_MILLI,
    }))
    const isInside = (note: ContextNote, span: (typeof spans)[number]) => note.timeUs >= span.fromUs && note.timeUs <= span.toUs
    const covered = notes.filter((note) => note.grade !== null && spans.some((span) => isInside(note, span)))
    return {
        gauge_drop_contexts: spans.map((span) => {
            const inside = notes.filter((note) => isInside(note, span))
            const npsSum = inside.reduce((sum, note) => sum + note.nps2, 0)
            return {
                from_ms: span.drop.from_ms,
                to_ms: span.drop.to_ms,
                chart_notes: inside.length,
                bp: inside.filter((note) => isBreakGrade(note.grade)).length,
                mean_nps: inside.length > 0 ? roundToDecimals(npsSum / inside.length, NPS_SUMMARY_DECIMALS) : 0,
                top_tags: topTagsOf(countTagsOf(inside.map((note) => note.tags)), CONTEXT_TOP_TAGS),
            }
        }),
        gauge_drop_tag_totals: tagCountRecordOf(countTagsOf(covered.map((note) => note.tags))),
        gauge_drop_notes: covered.length,
    }
}

const SOFLAN_PHASE = { BEFORE: 'before', AFTER: 'after', SETTLED: 'settled' } as const

/** The first index of ascending values greater than `value`, or the length. */
const firstIndexAfter = (sorted: readonly number[], value: number) => {
    let low = 0
    let high = sorted.length
    while (low < high) {
        const middle = (low + high) >> 1
        if (sorted[middle] <= value) low = middle + 1
        else high = middle
    }
    return low
}

/**
 * The phase of the nearest soflan event a time is in (plan 2026-11-04 2.2.7), null when it is in none: `before` from SOFLAN_PHASE_MS
 * before the event up to it, `after` from the event through its stop and SOFLAN_PHASE_MS more, `settled` from there up to
 * SOFLAN_SETTLED_MS past the stop. A time in the phases of two events belongs to the nearer one, the earlier one between equals.
 */
const createSoflanPhaseLookup = (events: readonly { tUs: number; stopUs: number }[]) => {
    const sorted = events.toSorted((a, b) => a.tUs - b.tUs)
    const times = sorted.map((event) => event.tUs)
    const phaseUs = SOFLAN_PHASE_MS * MICROS_PER_MILLI
    const settledUs = SOFLAN_SETTLED_MS * MICROS_PER_MILLI
    const reachUs = sorted.reduce((longest, event) => Math.max(longest, event.stopUs), 0) + settledUs
    return (timeUs: number) => {
        const next = firstIndexAfter(times, timeUs)
        let earlier: (typeof sorted)[number] | null = null
        for (let at = next - 1; at >= 0 && sorted[at].tUs >= timeUs - reachUs; at -= 1) {
            if (timeUs < sorted[at].tUs + sorted[at].stopUs + settledUs) {
                earlier = sorted[at]
                break
            }
        }
        const later = next < sorted.length && sorted[next].tUs - timeUs <= phaseUs ? sorted[next] : null
        if (earlier && (!later || timeUs - earlier.tUs <= later.tUs - timeUs)) {
            return timeUs < earlier.tUs + earlier.stopUs + phaseUs ? SOFLAN_PHASE.AFTER : SOFLAN_PHASE.SETTLED
        }
        return later ? SOFLAN_PHASE.BEFORE : null
    }
}

/**
 * The per-play figures of plan 4.3 from the notes as the judge engine left them, on the lanes the player saw (`layout: "played"`):
 * judgements overall and by tag, lane and note density; for each of SEGMENT_COUNT equal parts of the play its time span, its judged
 * objects (`chart_notes`, reached or not), the notes that took a judgement with their EX score, BP, combo breaks and FAST/SLOW (GREAT
 * to BAD) and its SEGMENT_TOP_TAGS most frequent tags; FAST/SLOW by grade and tag, the timing of PGREAT and GREAT overall and by lane,
 * and the first and last third's EX ratios. An unjudged LN end is left out. A third
 * with no judged note (nothing in it, or the play ended before it) has a null EX ratio and so a null `fatigue.delta`.
 *
 * Since PLAY_ANALYSIS_VERSION 3 (plan 2026-11-04 2.2) also, in forms that add up across plays (`CompactCell`, `TimingMoments`): the
 * notes by local NPS bin with their burst and sustained parts, the peak NPS and rows per second, timing moments by tag, lane and
 * hand, judgements by hand, BP by lane, the BAD/POOR runs, what the chart held around each gauge drop (`gaugeDropsMs`), judgements
 * by tag pair and by a lone combo tag, by BPM band and by soflan phase, and each training chart section's (`trainingSections`, null
 * when the chart is not one). The local NPS is read over the notes (`localDensitiesOf`), not the play's length, so it is the chart's.
 */
export const computePlayMetrics = (input: PlayMetricsInput) => {
    const tagLists = classifyPattern(input)
    const judged = input.notes.flatMap((note, index) => {
        const grade = GRADE_OF_STATE[note.state]
        return isJudgedNote(note) && grade ? [{ note, grade, tags: tagLists[index], index }] : []
    })
    const lastUs = input.notes.reduce((latest, note) => Math.max(latest, note.timeUs), 0)
    const lengthUs = input.lengthUs > 0 ? input.lengthUs : lastUs + 1
    const segmentOf = (timeUs: number) => Math.min(SEGMENT_COUNT - 1, Math.max(0, Math.floor((timeUs * SEGMENT_COUNT) / lengthUs)))

    const overall = emptyTally()
    for (const { note, grade } of judged) addNote(overall, grade, note.errUs)

    const windowCounts = densityWindowCounts(input.notes)
    const segmentEdgeMs = (index: number) => Math.floor((index * lengthUs) / SEGMENT_COUNT / MICROS_PER_MILLI)
    const segments = Array.from({ length: SEGMENT_COUNT }, (_, index) => ({
        id: segmentId(index),
        from_ms: segmentEdgeMs(index),
        to_ms: segmentEdgeMs(index + 1),
        chart_notes: 0,
        notes: 0,
        ex_score: 0,
        bp: 0,
        combo_breaks: 0,
        fast: 0,
        slow: 0,
    }))
    const segmentTags = Array.from({ length: SEGMENT_COUNT }, () => new Map<PatternTag, number>())
    input.notes.forEach((note, index) => {
        if (!isJudgedNote(note)) return
        const at = segmentOf(note.timeUs)
        segments[at].chart_notes += 1
        for (const tag of tagLists[index]) segmentTags[at].set(tag, (segmentTags[at].get(tag) ?? 0) + 1)
    })
    let combo = 0
    for (const { note, grade } of judged) {
        const segment = segments[segmentOf(note.timeUs)]
        segment.notes += 1
        segment.ex_score += EX_OF_GRADE[grade]
        if (grade !== 'pgreat' && grade !== 'poor') {
            if (note.errUs >= 0) segment.fast += 1
            else segment.slow += 1
        }
        if (grade === 'bad' || grade === 'poor') {
            segment.bp += 1
            if (combo > 0) segment.combo_breaks += 1
            combo = 0
        } else {
            combo += 1
        }
    }
    for (const timeUs of input.emptyPoorTimesUs) segments[segmentOf(timeUs)].bp += 1

    const firstThird = emptyTally()
    const lastThird = emptyTally()
    for (const { note, grade } of judged) {
        if (note.timeUs * PLAY_THIRDS < lengthUs) addNote(firstThird, grade, note.errUs)
        if (note.timeUs * PLAY_THIRDS >= lengthUs * (PLAY_THIRDS - 1)) addNote(lastThird, grade, note.errUs)
    }
    const first = finishTally(firstThird)
    const last = finishTally(lastThird)

    const timed = judged.filter(({ grade }) => grade === 'pgreat' || grade === 'great')

    const byTag = tallyGroups<PatternTag>(
        judged.flatMap(({ note, grade, tags }) => tags.map((tag): [PatternTag, Grade, number] => [tag, grade, note.errUs])),
    )

    const densities = localDensitiesOf(input.notes)
    const npsCells = new Map<number, { all: CompactCell; burst: CompactCell; sustained: CompactCell }>()
    for (const { note, grade, index } of judged) {
        const density = densities[index]
        const bin = npsBinOf(density.nps2)
        const cells = npsCells.get(bin) ?? { all: emptyCompactCell(), burst: emptyCompactCell(), sustained: emptyCompactCell() }
        npsCells.set(bin, cells)
        addToCell(cells.all, grade, note.errUs)
        if (density.context === DENSITY_CONTEXT.BURST) addToCell(cells.burst, grade, note.errUs)
        if (density.context === DENSITY_CONTEXT.SUSTAINED) addToCell(cells.sustained, grade, note.errUs)
    }

    const cellNoteOf = ({ note, grade }: (typeof judged)[number]) => ({ grade, errUs: note.errUs })
    const tagGroups = cellsByKey(judged.flatMap((entry) => entry.tags.map((tag) => [tag, cellNoteOf(entry)] as const)))
    const laneGroups = cellsByKey(judged.map((entry) => [laneNameOf(entry.note.lane), cellNoteOf(entry)] as const))
    const handGroups = cellsByKey(
        judged.flatMap((entry) => {
            const hand = SP7_LANE_HAND_KEYS.at(entry.note.lane)
            return hand ? [[hand, cellNoteOf(entry)] as const] : []
        }),
    )
    const tempoLookup = createTempoLookup(input.tempo, input.initBpm)
    const bpmGroups = cellsByKey(judged.map((entry) => [bpmBandOf(tempoLookup.bpmAt(entry.note.timeUs)), cellNoteOf(entry)] as const))

    const soflanEvents = soflanEventsOf(input.tempo, input.initBpm)
    const soflanPhaseOf = createSoflanPhaseLookup(soflanEvents)
    const soflanPhases = { before: emptyCompactCell(), after: emptyCompactCell(), settled: emptyCompactCell() }
    for (const { note, grade } of judged) {
        const phase = soflanPhaseOf(note.timeUs)
        if (phase) addToCell(soflanPhases[phase], grade, note.errUs)
    }

    const contextNotes = input.notes.flatMap((note, index) =>
        isJudgedNote(note)
            ? [{ timeUs: note.timeUs, tags: tagLists[index], nps2: densities[index].nps2, grade: GRADE_OF_STATE[note.state] ?? null }]
            : [],
    )
    const trainingSectionOf = (section: TrainingSectionSpan) => {
        const isInSection = (timeUs: number) => timeUs >= section.from_us && timeUs < section.to_us
        const cell = emptyCompactCell()
        const moments = emptyTimingMoments()
        for (const { note, grade } of judged) {
            if (!isInSection(note.timeUs)) continue
            addToCell(cell, grade, note.errUs)
            addToMoments(moments, grade, note.errUs)
        }
        return {
            index: section.index,
            kind: section.kind,
            focus_key: section.focus_key,
            from_ms: Math.floor(section.from_us / MICROS_PER_MILLI),
            to_ms: Math.floor(section.to_us / MICROS_PER_MILLI),
            chart_notes: contextNotes.filter((note) => isInSection(note.timeUs)).length,
            ...cell,
            moments: finishMoments(moments),
        }
    }

    return {
        analysis_version: ANALYSIS_VERSION,
        layout: 'played' as const,
        overall: finishTally(overall),
        by_tag: Object.fromEntries(PATTERN_TAGS.filter((tag) => byTag[tag]).map((tag) => [tag, byTag[tag]])),
        by_lane: tallyGroups(
            judged.map(({ note, grade }): [string, Grade, number] => [SP7_LANE_NAMES[note.lane] ?? String(note.lane), grade, note.errUs]),
        ),
        by_density: tallyGroups(
            judged.map(({ note, grade }): [string, Grade, number] => [
                densityBucketOf(windowCounts[densityWindowOf(note.timeUs)] ?? 0),
                grade,
                note.errUs,
            ]),
        ),
        segments: segments.map((segment, index) => ({
            ...segment,
            top_tags: [...segmentTags[index].entries()]
                .toSorted(([leftTag, left], [rightTag, right]) => right - left || PATTERN_TAGS.indexOf(leftTag) - PATTERN_TAGS.indexOf(rightTag))
                .slice(0, SEGMENT_TOP_TAGS)
                .map(([tag, notes]) => ({ tag, notes })),
        })),
        empty_poor: input.emptyPoorTimesUs.length,
        timing: summarizeTiming(timed.map(({ note }) => note.errUs)),
        timing_by_lane: Object.fromEntries(
            [...Map.groupBy(timed, ({ note }) => SP7_LANE_NAMES[note.lane] ?? String(note.lane)).entries()].map(([lane, notes]) => [
                lane,
                {
                    notes: notes.length,
                    mean_us: Math.round(notes.reduce((sum, { note }) => sum + note.errUs, 0) / notes.length),
                    moments: finishMoments(laneGroups.get(lane)?.moments ?? emptyTimingMoments()),
                },
            ]),
        ),
        fatigue: {
            first_third_ex_ratio: first.ex_ratio,
            last_third_ex_ratio: last.ex_ratio,
            delta: first.ex_ratio === null || last.ex_ratio === null ? null : roundRatio(last.ex_ratio - first.ex_ratio),
        },
        by_nps: Object.fromEntries(
            [...npsCells.entries()]
                .toSorted(([left], [right]) => left - right)
                .map(([bin, { all, burst, sustained }]) => [npsBinKeyOf(bin), { ...all, burst, sustained }]),
        ),
        peak_nps: peakNpsOf(input.notes),
        peak_rps_2s: peakRowsPerSecondOf(input.notes),
        timing_by_tag: momentsRecordOf(PATTERN_TAGS, tagGroups),
        by_hand: cellRecordOf(HAND_KEYS, handGroups),
        timing_by_hand: momentsRecordOf(HAND_KEYS, handGroups),
        lane_bp: Object.fromEntries([...laneGroups.entries()].map(([lane, { cell }]) => [lane, { notes: cell.notes, bp: cell.bp }])),
        bp_runs: computeBpRuns(judged.map(({ note, grade, tags, index }) => ({ timeUs: note.timeUs, grade, tags, nps2: densities[index].nps2 }))),
        ...gaugeDropFiguresOf(input.gaugeDropsMs, contextNotes),
        ...computeTagCombos(judged.map((entry) => ({ ...cellNoteOf(entry), tags: entry.tags }))),
        by_bpm_band: cellRecordOf(BPM_BANDS, bpmGroups),
        soflan_phase: { events: soflanEvents.length, ...soflanPhases },
        training_sections: input.trainingSections ? input.trainingSections.map(trainingSectionOf) : null,
    }
}

export type PlayMetrics = ReturnType<typeof computePlayMetrics>

/** How the run ended, read off its trace: played to its end, ended by an empty gauge, or left before its end (Escape, quit). */
export const RUN_END = { FINISHED: 'finished', GAUGE_EMPTY: 'gauge_empty', LEFT: 'left' } as const
export type RunEnd = (typeof RUN_END)[keyof typeof RUN_END]

/** What one gauge says about the run: it would have cleared, it failed (or would have), or the log cannot tell. */
export const GAUGE_OUTCOME = { CLEARED: 'cleared', FAILED: 'failed', UNKNOWN: 'unknown' } as const

/** One gauge's history as the client logs it: a value every `intervalMs`, between `min` and `max`, clearing at `border` or above. */
export type GaugeCurve = { intervalMs: number; values: readonly number[]; min: number; max: number; border: number }

/** The run a set of gauge histories belongs to: how it ended, when an empty gauge ended it, and which gauge was played. */
export type GaugeRun = { end: RunEnd; failedAtMs: number | null; selected: number }

/**
 * The samples of a curve the run really logged. A run ended by an empty gauge has every gauge filled with zeros from the failure on
 * (`GaugeLogRecorder::closed`, rbms-play), so only the samples up to and including the failure instant are values.
 */
export const liveGaugeValues = (curve: GaugeCurve, run: Pick<GaugeRun, 'end' | 'failedAtMs'>) => {
    if (run.end !== RUN_END.GAUGE_EMPTY || run.failedAtMs === null) return curve.values
    return curve.values.slice(0, Math.floor(run.failedAtMs / curve.intervalMs) + 1)
}

/**
 * Where the gauge fell GAUGE_DROP_MIN_RATIO of its range or more inside GAUGE_DROP_WINDOW_MS. Neighbouring samples that each close
 * such a fall join into one drop, reported from the window's last highest sample to the sample where the fall was deepest. The server reads the
 * client's curve and never computes a gauge itself.
 */
export const findGaugeDrops = (curve: GaugeCurve) => {
    const span = Math.max(1, Math.round(GAUGE_DROP_WINDOW_MS / curve.intervalMs))
    const threshold = GAUGE_DROP_MIN_RATIO * (curve.max - curve.min)
    const marks = curve.values.map((value, index) => {
        const window = curve.values.slice(Math.max(0, index - span), index + 1)
        const peak = window.reduce((high, sample) => Math.max(high, sample), value)
        return { index, peakIndex: Math.max(0, index - span) + window.lastIndexOf(peak), drop: peak - value }
    })
    const drops: { from_ms: number; to_ms: number; drop: number }[] = []
    let open: (typeof marks)[number] | null = null
    for (const mark of marks) {
        const isDrop = threshold > 0 && mark.drop >= threshold
        if (isDrop && (!open || mark.drop > open.drop)) open = mark
        if (isDrop || !open) continue
        drops.push({ from_ms: open.peakIndex * curve.intervalMs, to_ms: open.index * curve.intervalMs, drop: roundRatio(open.drop) })
        open = null
    }
    if (open) drops.push({ from_ms: open.peakIndex * curve.intervalMs, to_ms: open.index * curve.intervalMs, drop: roundRatio(open.drop) })
    return drops
}

const isSurvivalGauge = (curve: GaugeCurve) => curve.min === 0 && curve.border === 0

const gaugeOutcomeOf = (curve: GaugeCurve, live: readonly number[], run: GaugeRun, index: number) => {
    if (run.end === RUN_END.LEFT) return GAUGE_OUTCOME.FAILED
    const reachedMin = live.some((value) => value <= curve.min)
    if (run.end === RUN_END.GAUGE_EMPTY) {
        if (index === run.selected || (isSurvivalGauge(curve) && reachedMin)) return GAUGE_OUTCOME.FAILED
        return GAUGE_OUTCOME.UNKNOWN
    }
    const end = live.at(-1) ?? curve.min
    const isAlive = isSurvivalGauge(curve) ? end > 0 : end >= curve.border
    return isAlive ? GAUGE_OUTCOME.CLEARED : GAUGE_OUTCOME.FAILED
}

/**
 * How each logged gauge ended, for "what if another gauge" (plan 4.3). The server reads the client's curves and never runs a gauge.
 * A run played to its end: a survival gauge (HARD, EXHARD, HAZARD and the CLASS gauges have min 0 and border 0, `gauge_tables.rs`)
 * clears when it ends above 0, the others when they end at `border` or above. A run left before its end is stored as FAILED, so no
 * gauge clears. A run ended by an empty gauge is read up to the failure only (`liveGaugeValues`): the played gauge failed, a survival
 * gauge that had already emptied would have failed too, and every other gauge is `unknown` (`cleared: null`), its `end` the value at
 * the failure.
 */
export const summarizeGaugeCurves = (curves: readonly GaugeCurve[], run: GaugeRun) =>
    curves.map((curve, index) => {
        const live = liveGaugeValues(curve, run)
        const outcome = gaugeOutcomeOf(curve, live, run, index)
        return {
            end: roundRatio(live.at(-1) ?? curve.min),
            cleared: outcome === GAUGE_OUTCOME.UNKNOWN ? null : outcome === GAUGE_OUTCOME.CLEARED,
            reached_min: live.some((value) => value <= curve.min),
            outcome,
        }
    })

export type GaugeCurveSummary = ReturnType<typeof summarizeGaugeCurves>[number]

/**
 * A green number as plan Q11 shows it, worked out from a score row: its `options.green_number` is the lane travel time in whole
 * milliseconds (the client's float, rounded), so the skin's GREEN value (that time times GREEN_DISPLAY_FACTOR) can be off by one
 * here. It is marked `is_estimated`; a trace holds the exact value (`readTraceGreenNumber`).
 */
export const describeGreenNumber = (laneTimeMs: number) => ({
    green: Math.round(laneTimeMs * GREEN_DISPLAY_FACTOR),
    lane_ms: Math.round(laneTimeMs),
    is_estimated: true,
})

/** The green number of a trace's `speed.end` (`green`, `green_ms`): the client worked it out the way the skin does, so it is used as it is. */
export const readTraceGreenNumber = (trace: { green: number; greenMs: number }) => ({
    green: trace.green,
    lane_ms: trace.greenMs,
    is_estimated: false,
})
