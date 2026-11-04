import { BPM_BAND_EDGES, BPM_BANDS, RATIO_DECIMALS } from '@server/service/domain/analysis/analysis-constants'
import { PATTERN_TAGS, type PatternTag } from '@server/service/domain/analysis/pattern-tags'

/** The judgements a note can take, best first. */
export const GRADES = ['pgreat', 'great', 'good', 'bad', 'poor'] as const
export type Grade = (typeof GRADES)[number]

export const EX_PER_PGREAT = 2
export const EX_PER_GREAT = 1
export const EX_OF_GRADE: Record<Grade, number> = { pgreat: EX_PER_PGREAT, great: EX_PER_GREAT, good: 0, bad: 0, poor: 0 }

const MICROS_PER_MILLI = 1000

/**
 * The judgements of a group of notes in a form that adds up across plays (plan 2026-11-04 2.2): the notes that took a judgement
 * (POOR included), their EX score, BAD plus POOR, and the GREAT to BAD notes hit early (`fast`, error 0 or more) and late (`slow`).
 */
export type CompactCell = { notes: number; ex_score: number; bp: number; fast: number; slow: number }

/**
 * The timing errors of the PGREAT to BAD notes of a group (POOR has none) as sums that add up across plays: the count, the sum in
 * microseconds, the sum of squares in square milliseconds, and the count and sum of the early (error 0 or more) and late notes.
 */
export type TimingMoments = { n: number; sum_us: number; sumsq_ms2: number; fast_n: number; fast_sum_us: number; slow_n: number; slow_sum_us: number }

/** A value kept to `decimals` places. */
export const roundToDecimals = (value: number, decimals: number) => {
    const scale = 10 ** decimals
    return Math.round(value * scale) / scale
}

export const emptyCompactCell = () => ({ notes: 0, ex_score: 0, bp: 0, fast: 0, slow: 0 }) satisfies CompactCell

export const addToCell = (cell: CompactCell, grade: Grade, errUs: number) => {
    cell.notes += 1
    cell.ex_score += EX_OF_GRADE[grade]
    if (grade === 'bad' || grade === 'poor') cell.bp += 1
    if (grade === 'pgreat' || grade === 'poor') return
    if (errUs >= 0) cell.fast += 1
    else cell.slow += 1
}

export const emptyTimingMoments = () =>
    ({ n: 0, sum_us: 0, sumsq_ms2: 0, fast_n: 0, fast_sum_us: 0, slow_n: 0, slow_sum_us: 0 }) satisfies TimingMoments

export const addToMoments = (moments: TimingMoments, grade: Grade, errUs: number) => {
    if (grade === 'poor') return
    moments.n += 1
    moments.sum_us += errUs
    moments.sumsq_ms2 += (errUs / MICROS_PER_MILLI) ** 2
    if (errUs >= 0) {
        moments.fast_n += 1
        moments.fast_sum_us += errUs
    } else {
        moments.slow_n += 1
        moments.slow_sum_us += errUs
    }
}

/** The moments as stored: sums of microseconds to whole microseconds, the sum of squares to RATIO_DECIMALS places. */
export const finishMoments = (moments: TimingMoments) =>
    ({
        ...moments,
        sum_us: Math.round(moments.sum_us),
        sumsq_ms2: roundToDecimals(moments.sumsq_ms2, RATIO_DECIMALS),
        fast_sum_us: Math.round(moments.fast_sum_us),
        slow_sum_us: Math.round(moments.slow_sum_us),
    }) satisfies TimingMoments

/** One judged note as the cells read it. */
export type CellNote = { grade: Grade; errUs: number }

/** The cell and timing moments of each key, in the order the keys first come. */
export const cellsByKey = <Key extends string>(entries: Iterable<readonly [Key, CellNote]>) => {
    const cells = new Map<Key, { cell: CompactCell; moments: TimingMoments }>()
    for (const [key, { grade, errUs }] of entries) {
        const group = cells.get(key) ?? { cell: emptyCompactCell(), moments: emptyTimingMoments() }
        cells.set(key, group)
        addToCell(group.cell, grade, errUs)
        addToMoments(group.moments, grade, errUs)
    }
    return cells
}

/** The cells of the groups as a record, in the order of `keys`, leaving out keys with no group. */
export const cellRecordOf = <Key extends string>(keys: readonly Key[], groups: ReturnType<typeof cellsByKey<Key>>) =>
    Object.fromEntries(
        keys.flatMap((key) => {
            const group = groups.get(key)
            return group ? [[key, group.cell] as const] : []
        }),
    )

/** The stored timing moments of the groups as a record, in the order of `keys`, leaving out keys with no group. */
export const momentsRecordOf = <Key extends string>(keys: readonly Key[], groups: ReturnType<typeof cellsByKey<Key>>) =>
    Object.fromEntries(
        keys.flatMap((key) => {
            const group = groups.get(key)
            return group ? [[key, finishMoments(group.moments)] as const] : []
        }),
    )

/** How many of the tag lists carry each tag. */
export const countTagsOf = (tagLists: Iterable<readonly PatternTag[]>) => {
    const counts = new Map<PatternTag, number>()
    for (const tags of tagLists) {
        for (const tag of tags) counts.set(tag, (counts.get(tag) ?? 0) + 1)
    }
    return counts
}

/** A tag count as a record in PATTERN_TAGS order, leaving out tags that never came. */
export const tagCountRecordOf = (counts: ReadonlyMap<PatternTag, number>) =>
    Object.fromEntries(PATTERN_TAGS.flatMap((tag) => (counts.has(tag) ? [[tag, counts.get(tag) ?? 0] as const] : [])))

/** The `top` most frequent tags of a count, most first and PATTERN_TAGS order between equals. */
export const topTagsOf = (counts: ReadonlyMap<PatternTag, number>, top: number) =>
    [...counts.entries()]
        .toSorted(([leftTag, left], [rightTag, right]) => right - left || PATTERN_TAGS.indexOf(leftTag) - PATTERN_TAGS.indexOf(rightTag))
        .slice(0, top)
        .map(([tag, notes]) => ({ tag, notes }))

/** The BPM band a tempo falls in: one band per BPM_BAND_EDGES edge reached, the lowest below the first edge. */
export const bpmBandOf = (bpm: number) => BPM_BANDS[BPM_BAND_EDGES.filter((edge) => bpm >= edge).length]
