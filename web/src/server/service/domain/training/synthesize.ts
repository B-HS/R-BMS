import { CHART_PATTERN_LANE_COUNT } from '@server/dto/chart-pattern'
import type { TrainingLaneHint } from '@server/dto/training'
import { STAIR_MIN_RUN } from '@server/service/domain/analysis/analysis-constants'
import { greatestCommonDivisor, trainingLinesOf, type TrainingNote } from '@server/service/domain/training/bms-writer'
import { trainingBpmChangesOf, trainingBpmDefinitionsOf, trainingTimelineOf } from '@server/service/domain/training/chart-timing'
import { randomIndex, randomItem, sectionRandomOf, type RandomSource } from '@server/service/domain/training/prng'
import {
    cellsPerSecondOf,
    isOneOf,
    measureUsOf,
    roundNps,
    trainingGridsOf,
    trainingNotesPerRowOf,
    trainingPeakFactorOf,
    type TrainingSection,
    type TrainingSectionPlan,
    type TrainingSpecInput,
} from '@server/service/domain/training/spec'
import {
    MICROS_PER_SECOND,
    TRAINING_ADJACENT_GAP,
    TRAINING_BEAT_TICKS,
    TRAINING_BURST_HIGH,
    TRAINING_BURST_LOW,
    TRAINING_FOLLOW_SIXTEENTHS,
    TRAINING_HAND_LANES,
    TRAINING_LANE_CANDIDATES,
    TRAINING_LENGTH_US,
    TRAINING_LN_HOLD_BEATS,
    TRAINING_LN_KINDS,
    TRAINING_LN_MIN_CELLS,
    TRAINING_LN_RELEASE_BEATS,
    TRAINING_MEASURE_TICKS,
    TRAINING_PLAY_LEVEL_MAX,
    TRAINING_PLAY_LEVEL_MIN,
    TRAINING_RUN_ROWS,
    TRAINING_SCRATCH_KINDS,
    TRAINING_SCRATCH_LANE,
    TRAINING_SCRATCH_SOLO_KEY_SHARE,
    TRAINING_SEVEN_KEY_FALLBACK_LANE,
    TRAINING_SEVEN_KEY_LANES,
    TRAINING_SIXTEENTH_TICKS,
    TRAINING_TAIL_US,
    TRAINING_WIDE_MIN_GAP,
    type TrainingContinuousKind,
} from '@server/service/domain/training/training-constants'

/** A note as the synthesiser lays it: at an absolute tick (`measure × TRAINING_MEASURE_TICKS + tick`), a long note to its end tick. */
type Laid = { tick: number; lane: number; endTick: number | null }

type Span = { from: number; to: number }

/**
 * Which ticks each lane is taken at: a note takes its tick, a long note its whole body. A lane never has two notes at one tick
 * (the parser would keep the later one) and nothing inside a long note's body.
 */
const createBoard = () => {
    const spans: Span[][] = Array.from({ length: CHART_PATTERN_LANE_COUNT }, () => [])
    const laid: Laid[] = []
    const isFree = (lane: number, from: number, to = from) => spans[lane].every((span) => span.to < from || span.from > to)
    const lay = (lane: number, tick: number, endTick: number | null = null) => {
        spans[lane].push({ from: tick, to: endTick ?? tick })
        laid.push({ tick, lane, endTick })
    }
    const lengthen = (index: number, endTick: number) => {
        const note = laid[index]
        spans[note.lane] = spans[note.lane].map((span) =>
            span.from === note.tick && span.to === note.tick ? { from: note.tick, to: endTick } : span,
        )
        laid[index] = { ...note, endTick }
    }
    return { isFree, lay, lengthen, laid }
}

type Board = ReturnType<typeof createBoard>

type SectionContext = {
    section: TrainingSectionPlan
    random: RandomSource
    board: Board
    startTick: number
    endTick: number
    grid: number
    cellTicks: number
    cellCount: number
    candidates: readonly number[]
}

const HALF = 0.5

/** The coarsest grid whose cells come at least as fast as the section's busiest rows; the finest grid when none does. */
const gridOf = (section: TrainingSectionPlan) => {
    const rowsPerSecond = (section.target_nps / trainingNotesPerRowOf(section)) * trainingPeakFactorOf(section.kind)
    const grids = trainingGridsOf(section.kind)
    return grids.find((grid) => cellsPerSecondOf(grid, section.bpm) >= rowsPerSecond) ?? grids[grids.length - 1]
}

/** The share of cells that carry a row, so the rows meet the target on average. */
const rowShareOf = (context: SectionContext) =>
    Math.min(1, context.section.target_nps / trainingNotesPerRowOf(context.section) / cellsPerSecondOf(context.grid, context.section.bpm))

/** The cells that carry a row, spread evenly: each cell adds its share and takes a row once a whole one has gathered. */
const spreadCells = (cellCount: number, shareAt: (cell: number) => number) => {
    let gathered = 0
    return Array.from({ length: cellCount }, (_, cell) => cell).filter((cell) => {
        gathered += shareAt(cell)
        if (gathered < 1) return false
        gathered -= 1
        return true
    })
}

/** Runs of `rows` consecutive cells, one at the start of every cycle long enough for the target's rows a second. */
const runsOf = (context: SectionContext, rows: number) => {
    const share = rowShareOf(context)
    const cycle = Math.max(rows, Math.ceil(rows / share))
    return Array.from({ length: Math.ceil(context.cellCount / cycle) }, (_, run) => {
        const start = run * cycle
        return { start, cycle, cells: Array.from({ length: Math.min(rows, context.cellCount - start) }, (_, offset) => start + offset) }
    })
}

const tickOf = (context: SectionContext, cell: number) => context.startTick + cell * context.cellTicks

const preferredLanes = (pool: readonly number[], previous: number | undefined, hint: TrainingLaneHint) => {
    if (previous === undefined) return pool
    if (hint === 'wide') return pool.filter((lane) => Math.abs(lane - previous) >= TRAINING_WIDE_MIN_GAP)
    if (hint === 'adjacent') return pool.filter((lane) => Math.abs(lane - previous) === TRAINING_ADJACENT_GAP)
    return pool
}

/** A lane of `pool`, one the hint prefers after `previous` when there is one. */
const pickLane = (random: RandomSource, pool: readonly number[], previous: number | undefined, hint: TrainingLaneHint) => {
    const preferred = preferredLanes(pool, previous, hint)
    return randomItem(random, preferred.length > 0 ? preferred : pool)
}

/** `count` different items of `pool` in a random order (fewer when the pool is short). */
const takeRandom = (random: RandomSource, pool: readonly number[], count: number) => {
    let left = [...pool]
    const taken: number[] = []
    while (taken.length < count && left.length > 0) {
        const at = randomIndex(random, left.length)
        taken.push(left[at])
        left = left.toSpliced(at, 1)
    }
    return taken
}

const freeLanes = (context: SectionContext, lanes: readonly number[], from: number, to = from) =>
    lanes.filter((lane) => context.board.isFree(lane, from, to))

/**
 * Lays single keys one row at a time, never on the key of either of the two rows before when another is free, so no jack or trill
 * forms by chance. Returns the lane laid, or null when every candidate is taken at the tick.
 */
const createSingleLayer = (context: SectionContext, lanes: readonly number[] = context.candidates) => {
    let recent: number[] = []
    return (tick: number) => {
        const free = freeLanes(context, lanes, tick)
        const fresh = free.filter((lane) => !recent.includes(lane))
        const pool = fresh.length > 0 ? fresh : free
        if (pool.length === 0) return null
        const lane = pickLane(context.random, pool, recent.at(-1), context.section.lane_hint)
        context.board.lay(lane, tick)
        recent = [...recent, lane].slice(-2)
        return lane
    }
}

const layRows = (context: SectionContext, cells: readonly number[], layAt: (tick: number) => void) => {
    for (const cell of cells) layAt(tickOf(context, cell))
}

const runRowsOf = (kind: TrainingContinuousKind) => TRAINING_RUN_ROWS[kind]

const layStream = (context: SectionContext) => {
    const laySingle = createSingleLayer(context)
    for (const run of runsOf(context, runRowsOf('stream'))) layRows(context, run.cells, laySingle)
}

const layBurst = (context: SectionContext) => {
    const share = rowShareOf(context)
    const beatOf = (cell: number) => Math.floor((cell * context.cellTicks) / TRAINING_BEAT_TICKS)
    const cells = spreadCells(context.cellCount, (cell) => Math.min(1, share * (beatOf(cell) % 2 === 0 ? TRAINING_BURST_HIGH : TRAINING_BURST_LOW)))
    layRows(context, cells, createSingleLayer(context))
}

const layChordStream = (context: SectionContext) => {
    let previousRow: number[] = []
    layRows(
        context,
        spreadCells(context.cellCount, () => rowShareOf(context)),
        (tick) => {
            const free = freeLanes(context, context.candidates, tick)
            const chosen = takeRandom(
                context.random,
                free.filter((lane) => !previousRow.includes(lane)),
                context.section.chord_size,
            )
            const row = [
                ...chosen,
                ...takeRandom(
                    context.random,
                    free.filter((lane) => !chosen.includes(lane)),
                    context.section.chord_size - chosen.length,
                ),
            ]
            for (const lane of row) context.board.lay(lane, tick)
            previousRow = row
        },
    )
}

const pairsOf = (lanes: readonly number[], fits: (gap: number) => boolean) =>
    lanes.flatMap((first, index) => lanes.slice(index + 1).flatMap((second) => (fits(second - first) ? [[first, second] as const] : [])))

const layTrill = (context: SectionContext) => {
    const hint = context.section.lane_hint
    const fitsHint = (gap: number) => (hint === 'adjacent' ? gap === TRAINING_ADJACENT_GAP : hint !== 'wide' || gap >= TRAINING_WIDE_MIN_GAP)
    const hinted = pairsOf(context.candidates, fitsHint)
    const pairs = hinted.length > 0 ? hinted : pairsOf(context.candidates, () => true)
    for (const run of runsOf(context, runRowsOf('trill'))) {
        const [first, second] = randomItem(context.random, pairs)
        const lanes = context.random() < HALF ? [first, second] : [second, first]
        run.cells.forEach((cell, row) => {
            const tick = tickOf(context, cell)
            const lane = lanes[row % 2]
            if (context.board.isFree(lane, tick)) context.board.lay(lane, tick)
        })
    }
}

const layStair = (context: SectionContext) => {
    const low = Math.min(...context.candidates)
    const high = Math.max(...context.candidates)
    const shortest = Math.min(STAIR_MIN_RUN, high - low + 1)
    for (const run of runsOf(context, runRowsOf('stair'))) {
        const direction = context.random() < HALF ? 1 : -1
        const roomFrom = (lane: number) => (direction > 0 ? high - lane : lane - low) + 1
        const start = randomItem(
            context.random,
            context.candidates.filter((lane) => roomFrom(lane) >= shortest),
        )
        run.cells.slice(0, roomFrom(start)).forEach((cell, row) => {
            const tick = tickOf(context, cell)
            const lane = start + direction * row
            if (context.board.isFree(lane, tick)) context.board.lay(lane, tick)
        })
    }
}

/** Lays runs that repeat one set of keys on every row of the run, each run on another set when one is free. */
const layRepeatedRuns = (context: SectionContext, rows: number, size: number) => {
    let previous: number[] = []
    for (const run of runsOf(context, rows)) {
        const firstTick = tickOf(context, run.cells[0])
        const free = freeLanes(context, context.candidates, firstTick)
        const fresh = takeRandom(
            context.random,
            free.filter((lane) => !previous.includes(lane)),
            size,
        )
        const lanes = [
            ...fresh,
            ...takeRandom(
                context.random,
                free.filter((lane) => !fresh.includes(lane)),
                size - fresh.length,
            ),
        ]
        for (const cell of run.cells) {
            const tick = tickOf(context, cell)
            for (const lane of lanes) if (context.board.isFree(lane, tick)) context.board.lay(lane, tick)
        }
        previous = lanes
    }
}

const layJack = (context: SectionContext) => layRepeatedRuns(context, runRowsOf('jack'), 1)

const layChordJack = (context: SectionContext) => layRepeatedRuns(context, runRowsOf('chord_jack'), context.section.chord_size)

const layScratch = (context: SectionContext, tick: number) => {
    if (context.board.isFree(TRAINING_SCRATCH_LANE, tick)) context.board.lay(TRAINING_SCRATCH_LANE, tick)
}

const layScratchSolo = (context: SectionContext) => {
    const scratchCells = spreadCells(context.cellCount, () => rowShareOf(context))
    for (const cell of scratchCells) layScratch(context, tickOf(context, cell))
    const scratchTicks = scratchCells.map((cell) => tickOf(context, cell))
    const isAlone = (tick: number) => scratchTicks.every((scratchTick) => Math.abs(scratchTick - tick) > TRAINING_SIXTEENTH_TICKS)
    const keyCells = Array.from({ length: context.cellCount }, (_, cell) => cell).filter((cell) => isAlone(tickOf(context, cell)))
    const keyShare = keyCells.length > 0 ? Math.min(1, (scratchCells.length * TRAINING_SCRATCH_SOLO_KEY_SHARE) / keyCells.length) : 0
    const laySingle = createSingleLayer(context)
    for (const cell of keyCells) if (context.random() < keyShare) laySingle(tickOf(context, cell))
}

const layScratchMixed = (context: SectionContext) => {
    const laySingle = createSingleLayer(context)
    const follow = TRAINING_FOLLOW_SIXTEENTHS * TRAINING_SIXTEENTH_TICKS
    spreadCells(context.cellCount, () => rowShareOf(context)).forEach((cell, row) => {
        const tick = tickOf(context, cell)
        laySingle(tick)
        const later = tick + follow
        const isLater = row % 2 === 0 && later < context.endTick && context.board.isFree(TRAINING_SCRATCH_LANE, later)
        layScratch(context, isLater ? later : tick)
    })
}

const layScratchRun = (context: SectionContext) => {
    const rows = runRowsOf('scratch_run')
    const laySingle = createSingleLayer(context)
    for (const run of runsOf(context, rows)) {
        for (const cell of run.cells) layScratch(context, tickOf(context, cell))
        const gap = run.cycle - rows
        const keyCell = run.start + rows + Math.floor(gap / 2)
        if (gap > 0 && keyCell < context.cellCount) laySingle(tickOf(context, keyCell))
    }
}

const HANDS = ['left', 'right'] as const

const otherHand = (hand: (typeof HANDS)[number]) => (hand === 'left' ? 'right' : 'left')

const layLnHold = (context: SectionContext) => {
    const blockTicks = TRAINING_LN_HOLD_BEATS * TRAINING_BEAT_TICKS
    const firstHand = randomItem(context.random, HANDS)
    const lastEnd = context.endTick - context.cellTicks
    const blockCount = Math.ceil((context.endTick - context.startTick) / blockTicks)
    const handOfBlock = (block: number) => (block % 2 === 0 ? firstHand : otherHand(firstHand))
    for (let block = 0; block < blockCount; block += 1) {
        const tick = context.startTick + block * blockTicks
        const endTick = Math.min(tick + blockTicks, lastEnd)
        const lanes = freeLanes(context, TRAINING_HAND_LANES[handOfBlock(block)], tick, endTick)
        if (endTick > tick && lanes.length > 0) context.board.lay(randomItem(context.random, lanes), tick, endTick)
    }
    const layers = { left: createSingleLayer(context, TRAINING_HAND_LANES.left), right: createSingleLayer(context, TRAINING_HAND_LANES.right) }
    layRows(
        context,
        spreadCells(context.cellCount, () => rowShareOf(context)),
        (tick) => {
            layers[otherHand(handOfBlock(Math.floor((tick - context.startTick) / blockTicks)))](tick)
        },
    )
}

const layLnRelease = (context: SectionContext) => {
    const holdTicks = TRAINING_LN_RELEASE_BEATS * TRAINING_BEAT_TICKS
    const follow = TRAINING_FOLLOW_SIXTEENTHS * TRAINING_SIXTEENTH_TICKS
    let hand = randomItem(context.random, HANDS)
    layRows(
        context,
        spreadCells(context.cellCount, () => rowShareOf(context)),
        (tick) => {
            const endTick = tick + holdTicks
            const hitTick = endTick + follow
            if (hitTick >= context.endTick) return
            const holdLanes = freeLanes(context, TRAINING_HAND_LANES[hand], tick, endTick)
            if (holdLanes.length === 0) return
            const holdLane = randomItem(context.random, holdLanes)
            const hitLanes = freeLanes(
                context,
                TRAINING_HAND_LANES[hand].filter((lane) => lane !== holdLane),
                hitTick,
            )
            if (hitLanes.length === 0) return
            context.board.lay(holdLane, tick, endTick)
            context.board.lay(randomItem(context.random, hitLanes), hitTick)
            hand = otherHand(hand)
        },
    )
}

const layRest = (context: SectionContext) => {
    let next = 0
    layRows(
        context,
        spreadCells(context.cellCount, () => rowShareOf(context)),
        (tick) => {
            const order = context.candidates.map((_, offset) => (next + offset) % context.candidates.length)
            const at = order.find((index) => context.board.isFree(context.candidates[index], tick))
            if (at === undefined) return
            context.board.lay(context.candidates[at], tick)
            next = (at + 1) % context.candidates.length
        },
    )
}

const LAY_KIND = {
    stream: layStream,
    chord_stream: layChordStream,
    trill: layTrill,
    stair: layStair,
    jack: layJack,
    chord_jack: layChordJack,
    scratch_solo: layScratchSolo,
    scratch_mixed: layScratchMixed,
    scratch_run: layScratchRun,
    ln_hold: layLnHold,
    ln_release: layLnRelease,
    burst: layBurst,
    rest: layRest,
} satisfies Record<TrainingSectionPlan['kind'], (context: SectionContext) => void>

/** `scratch_ratio`: a scratch joins a key row with that chance, where the scratch is free. */
const addScratches = (context: SectionContext, from: number) => {
    const keyTicks = [...new Set(context.board.laid.slice(from).flatMap((note) => (note.lane === TRAINING_SCRATCH_LANE ? [] : [note.tick])))]
    for (const tick of keyTicks.toSorted((a, b) => a - b)) if (context.random() < context.section.scratch_ratio) layScratch(context, tick)
}

/** `ln_ratio`: a key hit becomes a long note with that chance, two cells to a beat long, when it ends inside the section with its lane free. */
const addLongNotes = (context: SectionContext, from: number) => {
    const shortest = Math.min(TRAINING_LN_MIN_CELLS * context.cellTicks, TRAINING_BEAT_TICKS)
    const lengths = Math.floor((TRAINING_BEAT_TICKS - shortest) / context.cellTicks) + 1
    const hits = context.board.laid.flatMap((note, index) =>
        index >= from && note.endTick === null && note.lane !== TRAINING_SCRATCH_LANE ? [index] : [],
    )
    for (const index of hits) {
        if (context.random() >= context.section.ln_ratio) continue
        const { tick, lane } = context.board.laid[index]
        const endTick = tick + shortest + randomIndex(context.random, lengths) * context.cellTicks
        if (endTick < context.endTick && context.board.isFree(lane, tick + 1, endTick)) context.board.lengthen(index, endTick)
    }
}

const laySection = (board: Board, seed: number, section: TrainingSectionPlan) => {
    const grid = gridOf(section)
    const context: SectionContext = {
        section,
        random: sectionRandomOf(seed, section.index),
        board,
        startTick: section.measure_from * TRAINING_MEASURE_TICKS,
        endTick: (section.measure_from + section.measure_count) * TRAINING_MEASURE_TICKS,
        grid,
        cellTicks: TRAINING_MEASURE_TICKS / grid,
        cellCount: section.measure_count * grid,
        candidates: TRAINING_LANE_CANDIDATES[section.lane_hint],
    }
    const from = board.laid.length
    LAY_KIND[section.kind](context)
    if (section.scratch_ratio > 0 && !isOneOf(TRAINING_SCRATCH_KINDS, section.kind)) addScratches(context, from)
    if (section.ln_ratio > 0 && !isOneOf(TRAINING_LN_KINDS, section.kind)) addLongNotes(context, from)
    return grid
}

/** The time of a tick as the chart's tempo gives it, not truncated: the count-in measure at the base tempo, then each section's. */
const createTickClock = (spec: TrainingSpecInput) => {
    const lastMeasure = spec.sections.reduce((last, section) => Math.max(last, section.measure_from + section.measure_count), 0)
    const bpmOfMeasure = Array.from({ length: lastMeasure + 1 }, (_, measure) => {
        const section = spec.sections.find(
            (candidate) => measure >= candidate.measure_from && measure < candidate.measure_from + candidate.measure_count,
        )
        return section?.bpm ?? spec.base_bpm
    })
    const startOfMeasure = bpmOfMeasure.reduce<number[]>((starts, bpm, measure) => [...starts, (starts[measure] ?? 0) + measureUsOf(bpm)], [0])
    return (tick: number) => {
        const measure = Math.min(lastMeasure, Math.floor(tick / TRAINING_MEASURE_TICKS))
        return startOfMeasure[measure] + ((tick - measure * TRAINING_MEASURE_TICKS) / TRAINING_MEASURE_TICKS) * measureUsOf(bpmOfMeasure[measure])
    }
}

/** A chart with no note on key 6 or 7 reads as 5 keys: the last row that has room gets key 7 (or 6). */
const ensureSevenKeys = (board: Board) => {
    const hasSevenKeys = board.laid.some((note) => note.endTick === null && (TRAINING_SEVEN_KEY_LANES as readonly number[]).includes(note.lane))
    if (hasSevenKeys) return
    const rows = [...new Set(board.laid.map((note) => note.tick))].toSorted((a, b) => b - a)
    const lanes = [TRAINING_SEVEN_KEY_FALLBACK_LANE, ...TRAINING_SEVEN_KEY_LANES.filter((lane) => lane !== TRAINING_SEVEN_KEY_FALLBACK_LANE)]
    for (const tick of rows) {
        const lane = lanes.find((candidate) => board.isFree(candidate, tick))
        if (lane !== undefined) {
            board.lay(lane, tick)
            return
        }
    }
}

const toNote = (tick: number, lane: number, kind: TrainingNote['kind']): TrainingNote => {
    const measure = Math.floor(tick / TRAINING_MEASURE_TICKS)
    const inMeasure = tick - measure * TRAINING_MEASURE_TICKS
    const divisor = greatestCommonDivisor(inMeasure, TRAINING_MEASURE_TICKS)
    return { measure, num: inMeasure / divisor, den: TRAINING_MEASURE_TICKS / divisor, lane, kind }
}

const KIND_ORDER = { normal: 0, ln_head: 1, ln_end: 2 } as const satisfies Record<TrainingNote['kind'], number>

const absoluteTickOf = (note: TrainingNote) => note.measure * TRAINING_MEASURE_TICKS + (note.num * TRAINING_MEASURE_TICKS) / note.den

/**
 * Lays the notes of a spec (plan 3.6) and fills what the layout decides: each section's times, grid, notes and actual NPS, and the
 * chart's play level. The same spec gives the same notes: every section draws from its own generator seeded by the spec's seed, and
 * the fields the synthesiser fills are not read. No note is left at or after the tail, and a chart with no key 6 or 7 gets one.
 */
export const synthesizeTraining = (input: TrainingSpecInput) => {
    const board = createBoard()
    const grids = input.sections.map((section) => laySection(board, input.seed, section))
    const clock = createTickClock(input)
    const cutoffUs = TRAINING_LENGTH_US - TRAINING_TAIL_US
    const kept = board.laid.filter((note) => clock(note.endTick ?? note.tick) < cutoffUs)
    const keptBoard = createBoard()
    for (const note of kept) keptBoard.lay(note.lane, note.tick, note.endTick)
    ensureSevenKeys(keptBoard)

    const notes = keptBoard.laid
        .flatMap((note) =>
            note.endTick === null
                ? [toNote(note.tick, note.lane, 'normal')]
                : [toNote(note.tick, note.lane, 'ln_head'), toNote(note.endTick, note.lane, 'ln_end')],
        )
        .toSorted((a, b) => absoluteTickOf(a) - absoluteTickOf(b) || a.lane - b.lane || KIND_ORDER[a.kind] - KIND_ORDER[b.kind])

    const timeline = trainingTimelineOf(trainingLinesOf(input, notes), input.base_bpm, trainingBpmDefinitionsOf(trainingBpmChangesOf(input)))
    const sections = input.sections.map((section, index): TrainingSection => {
        const startTick = section.measure_from * TRAINING_MEASURE_TICKS
        const endTick = (section.measure_from + section.measure_count) * TRAINING_MEASURE_TICKS
        const playable = keptBoard.laid.filter((note) => note.tick >= startTick && note.tick < endTick).length
        const fromUs = timeline.timeAt(section.measure_from)
        const toUs = timeline.timeAt(section.measure_from + section.measure_count)
        return {
            index: section.index,
            focus_key: section.focus_key,
            kind: section.kind,
            lane_hint: section.lane_hint,
            chord_size: section.chord_size,
            scratch_ratio: section.scratch_ratio,
            ln_ratio: section.ln_ratio,
            bpm: section.bpm,
            target_nps: section.target_nps,
            measure_from: section.measure_from,
            measure_count: section.measure_count,
            from_us: fromUs,
            to_us: toUs,
            grid: grids[index],
            actual_nps: toUs > fromUs ? roundNps((playable * MICROS_PER_SECOND) / (toUs - fromUs)) : 0,
            notes: playable,
            intent: section.intent,
        }
    })
    const peakNps = Math.max(0, ...sections.filter((section) => section.notes > 0).map((section) => section.actual_nps))
    const spec = {
        generator_version: input.generator_version,
        seed: input.seed,
        base_bpm: input.base_bpm,
        title: input.title,
        subtitle: input.subtitle,
        play_level: Math.min(TRAINING_PLAY_LEVEL_MAX, Math.max(TRAINING_PLAY_LEVEL_MIN, Math.round(peakNps))),
        sections,
        corrections: input.corrections,
    }
    return { spec, notes }
}
