import {
    CHORD_MIN_KEYS,
    CHORD_TOP_KEYS,
    DENSE_BASELINE_PERCENTILE,
    DENSE_PERCENTILE,
    DENSITY_WINDOW_US,
    FALLBACK_BPM,
    GAP_TOLERANCE_RATIO,
    HAND,
    JACK_MAX_GAP_S16,
    JACK_MIN_RUN,
    LN_FOLLOW_S16,
    MAX_DENSITY_WINDOWS,
    MINE_NEAR_S16,
    NEUTRAL_SCROLL,
    S16_US_PER_BPM,
    SCRATCH_LINK_S16,
    SCRATCH_RUN_GAP_S16,
    SCRATCH_RUN_MIN,
    SOFLAN_BPM_RATIO,
    SOFLAN_STOP_MIN_MS,
    SOFLAN_TAIL_MS,
    SP7_LANE_HANDS,
    SP7_SCRATCH_LANE,
    STAIR_MAX_GAP_S16,
    STAIR_MIN_RUN,
    STREAM_MAX_GAP_S16,
    STREAM_MIN_RUN,
    TRILL_MAX_GAP_S16,
    TRILL_MIN_RUN,
} from '@server/service/domain/analysis/analysis-constants'

export const PATTERN_TAGS = [
    'chord_2',
    'chord_3',
    'chord_4plus',
    'jack',
    'trill',
    'stair',
    'stream',
    'scratch_solo',
    'scratch_chord',
    'scratch_mixed',
    'with_scratch',
    'scratch_run',
    'ln_head',
    'ln_end',
    'ln_hold_hit',
    'ln_release_then_hit',
    'soflan',
    'dense',
    'mine_near',
] as const
export type PatternTag = (typeof PATTERN_TAGS)[number]

/** Tags that read only note times and kinds, so a play option that moves notes between lanes leaves them as they were (plan 4.5). */
export const LAYOUT_FREE_TAGS = [
    'chord_2',
    'chord_3',
    'chord_4plus',
    'stream',
    'ln_head',
    'ln_end',
    'soflan',
    'dense',
] as const satisfies readonly PatternTag[]

/** Tags that read lanes, which RANDOM-family options rearrange: on a chart's own layout they describe the layout with no option. */
export const LAYOUT_DEPENDENT_TAGS = PATTERN_TAGS.filter((tag) => !(LAYOUT_FREE_TAGS as readonly PatternTag[]).includes(tag))

const CHORD_TAGS = ['chord_2', 'chord_3', 'chord_4plus'] as const satisfies readonly PatternTag[]

export const SCRATCH_TAGS = ['scratch_solo', 'scratch_chord', 'scratch_mixed', 'with_scratch', 'scratch_run'] as const satisfies readonly PatternTag[]

/**
 * `ln_end` is a long note's end that is not judged (LN); `charge_end` is one that is (CN, HCN). A mine is never judged.
 */
export const PATTERN_NOTE_KIND = { NORMAL: 'normal', LN_HEAD: 'ln_head', LN_END: 'ln_end', CHARGE_END: 'charge_end', MINE: 'mine' } as const
export type PatternNoteKind = (typeof PATTERN_NOTE_KIND)[keyof typeof PATTERN_NOTE_KIND]

export type PatternNote = { timeUs: number; lane: number; kind: PatternNoteKind }

export type TempoPoint = { tUs: number; bpm: number; stopUs: number; scroll: number }

export type PatternInput = { notes: readonly PatternNote[]; tempo: readonly TempoPoint[]; initBpm: number }

export const isJudgedNote = (note: PatternNote) =>
    note.kind === PATTERN_NOTE_KIND.NORMAL || note.kind === PATTERN_NOTE_KIND.LN_HEAD || note.kind === PATTERN_NOTE_KIND.CHARGE_END

const isOnset = (note: PatternNote) => note.kind === PATTERN_NOTE_KIND.NORMAL || note.kind === PATTERN_NOTE_KIND.LN_HEAD

const isLongEnd = (note: PatternNote) => note.kind === PATTERN_NOTE_KIND.LN_END || note.kind === PATTERN_NOTE_KIND.CHARGE_END

const isScratchLane = (lane: number) => lane === SP7_SCRATCH_LANE

const usableBpm = (bpm: number, initBpm: number) => {
    if (bpm > 0) return bpm
    return initBpm > 0 ? initBpm : FALLBACK_BPM
}

/** The last index of a sorted array whose key is at most `value`, or -1. */
const lastIndexAtMost = (keys: readonly number[], value: number) => {
    let low = 0
    let high = keys.length - 1
    let found = -1
    while (low <= high) {
        const middle = (low + high) >> 1
        if (keys[middle] <= value) {
            found = middle
            low = middle + 1
        } else {
            high = middle - 1
        }
    }
    return found
}

/** The first index of a sorted array whose key is at least `value`, or the array's length. */
const firstIndexAtLeast = (keys: readonly number[], value: number) => {
    let low = 0
    let high = keys.length
    while (low < high) {
        const middle = (low + high) >> 1
        if (keys[middle] < value) low = middle + 1
        else high = middle
    }
    return low
}

/** The tempo and scroll speed at a chart time, read from the stated tempo points (each point holds until the next one). */
export const createTempoLookup = (tempo: readonly TempoPoint[], initBpm: number) => {
    const times = tempo.map((point) => point.tUs)
    const pointAt = (timeUs: number) => tempo[lastIndexAtMost(times, timeUs)]
    const bpmAt = (timeUs: number) => usableBpm(pointAt(timeUs)?.bpm ?? initBpm, initBpm)
    return {
        bpmAt,
        scrollAt: (timeUs: number) => pointAt(timeUs)?.scroll ?? NEUTRAL_SCROLL,
        sixteenthUsAt: (timeUs: number) => S16_US_PER_BPM / bpmAt(timeUs),
    }
}

type TempoLookup = ReturnType<typeof createTempoLookup>

const isWithinSixteenths = (lookup: TempoLookup, earlierUs: number, laterUs: number, sixteenths: number) =>
    laterUs - earlierUs <= sixteenths * lookup.sixteenthUsAt(laterUs) * (1 + GAP_TOLERANCE_RATIO)

type Row = { timeUs: number; keyNotes: number[]; scratchNotes: number[] }

const rowsOf = (notes: readonly PatternNote[], onsetIndexes: readonly number[]) => {
    const rows: Row[] = []
    for (const index of onsetIndexes) {
        const note = notes[index]
        const last = rows.at(-1)
        const row = last && last.timeUs === note.timeUs ? last : { timeUs: note.timeUs, keyNotes: [], scratchNotes: [] }
        if (row !== last) rows.push(row)
        if (isScratchLane(note.lane)) row.scratchNotes.push(index)
        else row.keyNotes.push(index)
    }
    return rows
}

/** Splits items into runs where each item links to the one before it, keeping runs of `minLength` or more. */
const linkedRuns = <T>(items: readonly T[], isLinked: (previous: T, next: T) => boolean, minLength: number) => {
    const runs: T[][] = []
    let run: T[] = []
    for (const item of items) {
        const previous = run.at(-1)
        if (previous !== undefined && isLinked(previous, item)) {
            run.push(item)
            continue
        }
        if (run.length >= minLength) runs.push(run)
        run = [item]
    }
    if (run.length >= minLength) runs.push(run)
    return runs
}

/**
 * Runs of single-key rows where each row continues the run by `continues(run, row)`. A row that breaks the run may start the next
 * one together with the row before it when `pairs(previous, row)` holds, so one row can close a run and open another.
 */
const singleKeyRuns = (
    rows: readonly Row[],
    continues: (run: readonly Row[], row: Row) => boolean,
    pairs: (previous: Row, row: Row) => boolean,
    minLength: number,
) => {
    const runs: Row[][] = []
    let run: Row[] = []
    const close = () => {
        if (run.length >= minLength) runs.push(run)
    }
    for (const row of rows) {
        if (row.keyNotes.length !== 1) {
            close()
            run = []
            continue
        }
        if (run.length > 0 && continues(run, row)) {
            run.push(row)
            continue
        }
        close()
        const previous = run.at(-1)
        run = previous && pairs(previous, row) ? [previous, row] : [row]
    }
    close()
    return runs
}

type Zone = { startUs: number; endUs: number }

const isTempoJump = (previousBpm: number, bpm: number) => {
    if (previousBpm <= 0 || bpm <= 0) return previousBpm !== bpm
    return Math.max(previousBpm, bpm) / Math.min(previousBpm, bpm) >= SOFLAN_BPM_RATIO
}

const MICROS_PER_MILLI = 1000

const soflanZones = (tempo: readonly TempoPoint[], initBpm: number) => {
    const zones: Zone[] = []
    let previousBpm = initBpm
    for (const point of tempo) {
        if (isTempoJump(previousBpm, point.bpm)) zones.push({ startUs: point.tUs, endUs: point.tUs + SOFLAN_TAIL_MS * MICROS_PER_MILLI })
        if (point.stopUs >= SOFLAN_STOP_MIN_MS * MICROS_PER_MILLI) {
            zones.push({ startUs: point.tUs, endUs: point.tUs + point.stopUs + SOFLAN_TAIL_MS * MICROS_PER_MILLI })
        }
        previousBpm = point.bpm
    }
    return zones
}

/** How many tempo jumps and long stops a chart has, by the soflan rules. */
export const countSoflanEvents = (tempo: readonly TempoPoint[], initBpm: number) => {
    let previousBpm = initBpm
    let jumps = 0
    let stops = 0
    for (const point of tempo) {
        if (isTempoJump(previousBpm, point.bpm)) jumps += 1
        if (point.stopUs >= SOFLAN_STOP_MIN_MS * MICROS_PER_MILLI) stops += 1
        previousBpm = point.bpm
    }
    return { jumps, stops }
}

/** The zones joined where they overlap, ordered by start, so a time is looked up with one search. */
const mergeZones = (zones: readonly Zone[]) => {
    const merged: Zone[] = []
    for (const zone of zones.toSorted((a, b) => a.startUs - b.startUs)) {
        const last = merged.at(-1)
        if (last && zone.startUs <= last.endUs) last.endUs = Math.max(last.endUs, zone.endUs)
        else merged.push({ ...zone })
    }
    return merged
}

const createZoneLookup = (zones: readonly Zone[]) => {
    const merged = mergeZones(zones)
    const starts = merged.map((zone) => zone.startUs)
    return (timeUs: number) => {
        const index = lastIndexAtMost(starts, timeUs)
        return index >= 0 && timeUs <= merged[index].endUs
    }
}

const isSameHand = (lane: number, otherLane: number) => {
    const hand = SP7_LANE_HANDS[lane]
    const otherHand = SP7_LANE_HANDS[otherLane]
    return hand === otherHand || hand === HAND.SHARED || otherHand === HAND.SHARED
}

/** The density window a time falls in. A time later than the longest allowed chart time falls in the last window. */
export const densityWindowOf = (timeUs: number) => Math.min(MAX_DENSITY_WINDOWS - 1, Math.floor(timeUs / DENSITY_WINDOW_US))

/** The judged-note count of each density window, indexed by window, at most MAX_DENSITY_WINDOWS long, so a bad time never makes a large array. */
export const densityWindowCounts = (notes: readonly PatternNote[]) => {
    const judgedWindows = notes.filter(isJudgedNote).map((note) => densityWindowOf(note.timeUs))
    if (judgedWindows.length === 0) return []
    const counts = new Array<number>(judgedWindows.reduce((latest, window) => Math.max(latest, window), 0) + 1).fill(0)
    for (const window of judgedWindows) counts[window] += 1
    return counts
}

const PERCENT = 100

/** The nearest-rank percentile of the window counts, the smallest count at least `percentile`% of the windows do not exceed. */
const nearestRankPercentile = (counts: readonly number[], percentile: number) => {
    const sorted = counts.toSorted((a, b) => a - b)
    return sorted[Math.max(0, Math.ceil((percentile / PERCENT) * sorted.length) - 1)] ?? 0
}

/**
 * Tags each note of a BEAT_7K layout by the v1 pattern rules (plan 4.2). The result has one tag list per input note, in the input's
 * order; a note that is not judged (a mine, an LN end) has none. Notes must be ordered by time and then lane. Gaps are measured in
 * local sixteenth notes of the tempo at the later note. Rows, chords, jacks, trills, stairs and streams are built from hits (normal
 * notes and LN heads); a judged CN or HCN end carries `ln_end` and the time-window tags. A jack links two hits of one lane only
 * when no other key row comes between them, so the notes of a trill are not jacks. A window is dense only when it also holds more
 * notes than the typical (median) window, so a chart of even density has no dense part.
 */
export const classifyPattern = (input: PatternInput) => {
    const { notes } = input
    const lookup = createTempoLookup(input.tempo, input.initBpm)
    const tags = notes.map(() => new Set<PatternTag>())
    const tagAll = (indexes: Iterable<number>, tag: PatternTag) => {
        for (const index of indexes) tags[index].add(tag)
    }

    const onsetIndexes = notes.flatMap((note, index) => (isOnset(note) ? [index] : []))
    const rows = rowsOf(notes, onsetIndexes)
    const keyRows = rows.filter((row) => row.keyNotes.length > 0)

    for (const row of keyRows) {
        if (row.keyNotes.length < CHORD_MIN_KEYS) continue
        tagAll(row.keyNotes, CHORD_TAGS[Math.min(row.keyNotes.length, CHORD_TOP_KEYS) - CHORD_MIN_KEYS])
    }

    const keyLaneOf = (row: Row) => notes[row.keyNotes[0]].lane
    const isRowGapWithin = (sixteenths: number) => (previous: Row, row: Row) => isWithinSixteenths(lookup, previous.timeUs, row.timeUs, sixteenths)

    const isTrillStep = isRowGapWithin(TRILL_MAX_GAP_S16)
    const trillPairs = (previous: Row, row: Row) => keyLaneOf(previous) !== keyLaneOf(row) && isTrillStep(previous, row)
    const trillRuns = singleKeyRuns(
        keyRows,
        (run, row) => {
            const beforePrevious = run.at(-2)
            return trillPairs(run[run.length - 1], row) && (!beforePrevious || keyLaneOf(beforePrevious) === keyLaneOf(row))
        },
        trillPairs,
        TRILL_MIN_RUN,
    )
    for (const run of trillRuns)
        tagAll(
            run.flatMap((row) => row.keyNotes),
            'trill',
        )

    const isStairStep = isRowGapWithin(STAIR_MAX_GAP_S16)
    const stairPairs = (previous: Row, row: Row) => Math.abs(keyLaneOf(row) - keyLaneOf(previous)) === 1 && isStairStep(previous, row)
    const stairRuns = singleKeyRuns(
        keyRows,
        (run, row) => {
            const previous = run[run.length - 1]
            if (!stairPairs(previous, row)) return false
            return run.length < 2 || keyLaneOf(row) - keyLaneOf(previous) === keyLaneOf(run[1]) - keyLaneOf(run[0])
        },
        stairPairs,
        STAIR_MIN_RUN,
    )
    for (const run of stairRuns)
        tagAll(
            run.flatMap((row) => row.keyNotes),
            'stair',
        )

    const keyOnsetIndexes = onsetIndexes.filter((index) => !isScratchLane(notes[index].lane))
    const keyRowOrder = new Map(keyRows.flatMap((row, order) => row.keyNotes.map((index): [number, number] => [index, order])))
    const keyLanes = [...new Set(keyOnsetIndexes.map((index) => notes[index].lane))]
    for (const laneOnsets of keyLanes.map((lane) => keyOnsetIndexes.filter((index) => notes[index].lane === lane))) {
        const jackRuns = linkedRuns(
            laneOnsets,
            (previous, next) =>
                keyRowOrder.get(next) === (keyRowOrder.get(previous) ?? 0) + 1 &&
                isWithinSixteenths(lookup, notes[previous].timeUs, notes[next].timeUs, JACK_MAX_GAP_S16),
            JACK_MIN_RUN,
        )
        for (const run of jackRuns) tagAll(run, 'jack')
    }

    const streamRuns = linkedRuns(keyRows, isRowGapWithin(STREAM_MAX_GAP_S16), STREAM_MIN_RUN)
    for (const run of streamRuns) {
        const plain = run
            .flatMap((row) => row.keyNotes)
            .filter((index) => !tags[index].has('jack') && !tags[index].has('trill') && !tags[index].has('stair'))
        tagAll(plain, 'stream')
    }

    const keyOnsetTimes = keyOnsetIndexes.map((index) => notes[index].timeUs)
    const keyRowTimes = new Set(keyRows.map((row) => row.timeUs))
    const scratchOnsets = onsetIndexes.filter((index) => isScratchLane(notes[index].lane))
    for (const index of scratchOnsets) {
        const { timeUs } = notes[index]
        if (keyRowTimes.has(timeUs)) {
            tags[index].add('scratch_chord')
            continue
        }
        const reachUs = SCRATCH_LINK_S16 * lookup.sixteenthUsAt(timeUs) * (1 + GAP_TOLERANCE_RATIO)
        const from = firstIndexAtLeast(keyOnsetTimes, timeUs - reachUs)
        const to = lastIndexAtMost(keyOnsetTimes, timeUs + reachUs)
        if (from > to) {
            tags[index].add('scratch_solo')
            continue
        }
        tags[index].add('scratch_mixed')
        tagAll(keyOnsetIndexes.slice(from, to + 1), 'with_scratch')
    }
    const scratchRuns = linkedRuns(
        scratchOnsets,
        (previous, next) => isWithinSixteenths(lookup, notes[previous].timeUs, notes[next].timeUs, SCRATCH_RUN_GAP_S16),
        SCRATCH_RUN_MIN,
    )
    for (const run of scratchRuns) tagAll(run, 'scratch_run')

    const openHeads = new Map<number, number>()
    const holds: { lane: number; startUs: number; endUs: number }[] = []
    notes.forEach((note, index) => {
        if (note.kind === PATTERN_NOTE_KIND.LN_HEAD) {
            tags[index].add('ln_head')
            openHeads.set(note.lane, note.timeUs)
            return
        }
        if (!isLongEnd(note)) return
        if (note.kind === PATTERN_NOTE_KIND.CHARGE_END) tags[index].add('ln_end')
        const startUs = openHeads.get(note.lane)
        if (startUs === undefined) return
        openHeads.delete(note.lane)
        holds.push({ lane: note.lane, startUs, endUs: note.timeUs })
    })
    const holdEnds = holds.toSorted((a, b) => a.endUs - b.endUs)
    const holdEndTimes = holdEnds.map((hold) => hold.endUs)
    const holdStarts = holds.toSorted((a, b) => a.startUs - b.startUs)
    let nextHold = 0
    let heldNow: typeof holds = []
    for (const index of onsetIndexes) {
        const { timeUs, lane } = notes[index]
        while (nextHold < holdStarts.length && holdStarts[nextHold].startUs < timeUs) {
            heldNow.push(holdStarts[nextHold])
            nextHold += 1
        }
        heldNow = heldNow.filter((hold) => hold.endUs > timeUs)
        if (heldNow.some((hold) => hold.lane !== lane)) tags[index].add('ln_hold_hit')
        const followUs = LN_FOLLOW_S16 * lookup.sixteenthUsAt(timeUs) * (1 + GAP_TOLERANCE_RATIO)
        const releases = holdEnds.slice(firstIndexAtLeast(holdEndTimes, timeUs - followUs), firstIndexAtLeast(holdEndTimes, timeUs))
        if (releases.some((hold) => isSameHand(lane, hold.lane))) tags[index].add('ln_release_then_hit')
    }

    const isInSoflanZone = createZoneLookup(soflanZones(input.tempo, input.initBpm))
    const windowCounts = densityWindowCounts(notes)
    const denseFrom = nearestRankPercentile(windowCounts, DENSE_PERCENTILE)
    const typicalCount = nearestRankPercentile(windowCounts, DENSE_BASELINE_PERCENTILE)
    const mineTimes = notes.filter((note) => note.kind === PATTERN_NOTE_KIND.MINE).map((note) => note.timeUs)
    notes.forEach((note, index) => {
        if (!isJudgedNote(note)) return
        if (isInSoflanZone(note.timeUs) || lookup.scrollAt(note.timeUs) !== NEUTRAL_SCROLL) tags[index].add('soflan')
        const windowCount = windowCounts[densityWindowOf(note.timeUs)]
        if (windowCount >= denseFrom && windowCount > typicalCount) tags[index].add('dense')
        const reachUs = MINE_NEAR_S16 * lookup.sixteenthUsAt(note.timeUs) * (1 + GAP_TOLERANCE_RATIO)
        const nearestMine = firstIndexAtLeast(mineTimes, note.timeUs - reachUs)
        if (nearestMine < mineTimes.length && mineTimes[nearestMine] <= note.timeUs + reachUs) tags[index].add('mine_near')
    })

    return tags.map((set) => PATTERN_TAGS.filter((tag) => set.has(tag)))
}
