import { CHART_PATTERN_LANE_COUNT } from '@server/dto/chart-pattern'
import type { TrainingLaneHint } from '@server/dto/training'
import { STAIR_MIN_RUN } from '@server/service/domain/analysis/analysis-constants'
import { localDensitiesOf } from '@server/service/domain/analysis/local-density'
import { greatestCommonDivisor, trainingLinesOf, type TrainingNote } from '@server/service/domain/training/bms-writer'
import { trainingBpmChangesOf, trainingBpmDefinitionsOf, trainingTimelineOf } from '@server/service/domain/training/chart-timing'
import {
    nearestRankOf,
    solveBodyMean,
    trainingCurveOf,
    trainingCurvePointsOf,
    trainingCurveSeedOf,
} from '@server/service/domain/training/density-curve'
import { randomIndex, randomItem, sectionRandomOf, type RandomSource } from '@server/service/domain/training/prng'
import {
    cellsPerSecondOf,
    isOneOf,
    isWarmupSection,
    measureUsOf,
    roundNps,
    trainingExtraNpsOf,
    trainingGridsOf,
    trainingMaxNpsOf,
    trainingNotesPerRowOf,
    trainingRunFillOf,
    trainingRunLengthOf,
    type TrainingSection,
    type TrainingSectionPlan,
    type TrainingSpecInput,
} from '@server/service/domain/training/spec'
import {
    MICROS_PER_SECOND,
    TRAINING_ADJACENT_GAP,
    TRAINING_BEAT_TICKS,
    TRAINING_BODY_PERCENTILE,
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
    TRAINING_LN_RELEASE_SHORTENINGS,
    TRAINING_MEAN_PASSES,
    TRAINING_MEAN_REFINE_RATIO,
    TRAINING_MEASURE_TICKS,
    TRAINING_NPS_FLOOR,
    TRAINING_P90_TOLERANCE,
    TRAINING_PLAY_LEVEL_MAX,
    TRAINING_PLAY_LEVEL_MIN,
    TRAINING_REST_NPS_MAX,
    TRAINING_REST_NPS_MIN,
    TRAINING_ROW_DEBT_MAX,
    TRAINING_RUN_GAP_KINDS,
    TRAINING_RUN_ROWS,
    TRAINING_SCRATCH_KINDS,
    TRAINING_SCRATCH_LANE,
    TRAINING_SCRATCH_SOLO_KEY_GAP_SIXTEENTHS,
    TRAINING_SCRATCH_SOLO_KEY_SHARE,
    TRAINING_SEVEN_KEY_FALLBACK_LANE,
    TRAINING_SEVEN_KEY_LANES,
    TRAINING_SIXTEENTH_TICKS,
    TRAINING_TAIL_US,
    TRAINING_WARMUP_DENSITY_RATIO,
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

/**
 * A section as its notes are laid: its cells (each measure on the coarsest grid that keeps up with the measure's busiest rows, M4),
 * the share of each cell that carries a row so the rows follow the density curve, and the cell length of each measure.
 */
type SectionContext = {
    section: TrainingSectionPlan
    random: RandomSource
    board: Board
    startTick: number
    endTick: number
    cells: readonly number[]
    shares: readonly number[]
    cellTicksAt: (tick: number) => number
    candidates: readonly number[]
}

const HALF = 0.5

/** The cells that carry a row: each cell adds its share and takes a row once a whole one has gathered (error diffusion). */
const spreadCells = (shares: readonly number[]) => {
    let gathered = 0
    return shares.flatMap((share, cell) => {
        gathered += share
        if (gathered < 1) return []
        gathered -= 1
        return [cell]
    })
}

/**
 * Lays a row on the cells the shares give, as `spreadCells` does, but a row `layAt` finds no room for is carried to the next cells
 * (at most TRAINING_ROW_DEBT_MAX at once), so a crowded cell does not lose its row (M6).
 */
const layRowsWithDebt = (context: SectionContext, layAt: (tick: number, row: number) => boolean) => {
    let gathered = 0
    let row = 0
    context.cells.forEach((tick, cell) => {
        gathered = Math.min(TRAINING_ROW_DEBT_MAX, gathered + context.shares[cell])
        if (gathered < 1 || !layAt(tick, row)) return
        gathered -= 1
        row += 1
    })
}

/**
 * Runs of consecutive cells, one starting at each group of rows the shares spread (M6): the run of `planOf().length` rows starts on
 * the cell of its group's first row, so the runs lay the rows the target asks for with no rounding lost to a fixed cycle. A run of a
 * kind of TRAINING_RUN_GAP_KINDS starts one empty cell after the one before at the soonest, so two runs never read as one.
 */
const runsOf = <T>(context: SectionContext, planOf: () => { length: number; data: T }) => {
    const rowCells = spreadCells(context.shares)
    const runs: { cells: number[]; data: T }[] = []
    const gap = isOneOf(TRAINING_RUN_GAP_KINDS, context.section.kind) ? 1 : 0
    let at = 0
    let nextFree = 0
    while (at < rowCells.length) {
        const { length, data } = planOf()
        const start = Math.max(rowCells[at], nextFree)
        const rows = Math.max(0, Math.min(length, rowCells.length - at, context.cells.length - start))
        if (rows > 0) runs.push({ cells: Array.from({ length: rows }, (_, offset) => start + offset), data })
        nextFree = start + rows + gap
        at += length
    }
    return runs
}

const fixedRuns = (context: SectionContext, rows: number) => runsOf(context, () => ({ length: rows, data: null }))

const tickOf = (context: SectionContext, cell: number) => context.cells[cell]

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

const runRowsOf = (kind: TrainingContinuousKind) => TRAINING_RUN_ROWS[kind]

const layStream = (context: SectionContext) => {
    const laySingle = createSingleLayer(context)
    for (const run of fixedRuns(context, runRowsOf('stream'))) for (const cell of run.cells) laySingle(tickOf(context, cell))
}

const laySingles = (context: SectionContext) => {
    const laySingle = createSingleLayer(context)
    layRowsWithDebt(context, (tick) => laySingle(tick) !== null)
}

const layChordStream = (context: SectionContext) => {
    let previousRow: number[] = []
    layRowsWithDebt(context, (tick) => {
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
        if (row.length > 0) previousRow = row
        return row.length > 0
    })
}

const pairsOf = (lanes: readonly number[], fits: (gap: number) => boolean) =>
    lanes.flatMap((first, index) => lanes.slice(index + 1).flatMap((second) => (fits(second - first) ? [[first, second] as const] : [])))

const layTrill = (context: SectionContext) => {
    const hint = context.section.lane_hint
    const fitsHint = (gap: number) => (hint === 'adjacent' ? gap === TRAINING_ADJACENT_GAP : hint !== 'wide' || gap >= TRAINING_WIDE_MIN_GAP)
    const hinted = pairsOf(context.candidates, fitsHint)
    const pairs = hinted.length > 0 ? hinted : pairsOf(context.candidates, () => true)
    for (const run of fixedRuns(context, runRowsOf('trill'))) {
        const [first, second] = randomItem(context.random, pairs)
        const lanes = context.random() < HALF ? [first, second] : [second, first]
        run.cells.forEach((cell, row) => {
            const tick = tickOf(context, cell)
            const lane = lanes[row % 2]
            if (context.board.isFree(lane, tick)) context.board.lay(lane, tick)
        })
    }
}

/** Stairs from a random key toward an end, each as long as the keys left that way (M6): every row the target asks for is laid. */
const layStair = (context: SectionContext) => {
    const low = Math.min(...context.candidates)
    const high = Math.max(...context.candidates)
    const shortest = Math.min(STAIR_MIN_RUN, high - low + 1)
    const runs = runsOf(context, () => {
        const direction = context.random() < HALF ? 1 : -1
        const roomFrom = (lane: number) => (direction > 0 ? high - lane : lane - low) + 1
        const start = randomItem(
            context.random,
            context.candidates.filter((lane) => roomFrom(lane) >= shortest),
        )
        return { length: roomFrom(start), data: { direction, start } }
    })
    for (const run of runs) {
        run.cells.forEach((cell, row) => {
            const tick = tickOf(context, cell)
            const lane = run.data.start + run.data.direction * row
            if (context.board.isFree(lane, tick)) context.board.lay(lane, tick)
        })
    }
}

/** Lays runs that repeat one set of keys on every row of the run, each run on another set when one is free. */
const layRepeatedRuns = (context: SectionContext, rows: number, size: number) => {
    let previous: number[] = []
    for (const run of fixedRuns(context, rows)) {
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
    if (!context.board.isFree(TRAINING_SCRATCH_LANE, tick)) return false
    context.board.lay(TRAINING_SCRATCH_LANE, tick)
    return true
}

/**
 * Scratches on the rows the target asks for (M6), a row turned into a key where it stands two sixteenths or more from the rows on
 * both sides (so no scratch near it reads as mixed), keys at most TRAINING_SCRATCH_SOLO_KEY_SHARE of the scratches.
 */
const layScratchSolo = (context: SectionContext) => {
    const rowTicks = spreadCells(context.shares).map((cell) => tickOf(context, cell))
    const gap = TRAINING_SCRATCH_SOLO_KEY_GAP_SIXTEENTHS * TRAINING_SIXTEENTH_TICKS
    const isApart = (row: number) =>
        (rowTicks[row - 1] === undefined || rowTicks[row] - rowTicks[row - 1] >= gap) &&
        (rowTicks[row + 1] === undefined || rowTicks[row + 1] - rowTicks[row] >= gap)
    const laySingle = createSingleLayer(context)
    let scratches = 0
    let keys = 0
    rowTicks.forEach((tick, row) => {
        const isKey = isApart(row) && keys + 1 <= TRAINING_SCRATCH_SOLO_KEY_SHARE * scratches
        if (isKey && laySingle(tick) !== null) {
            keys += 1
            return
        }
        if (layScratch(context, tick)) scratches += 1
    })
}

const layScratchMixed = (context: SectionContext) => {
    const laySingle = createSingleLayer(context)
    const follow = TRAINING_FOLLOW_SIXTEENTHS * TRAINING_SIXTEENTH_TICKS
    layRowsWithDebt(context, (tick, row) => {
        if (laySingle(tick) === null) return false
        const later = tick + follow
        const isLaterRoom = later < context.endTick
        const order = row % 2 === 0 && isLaterRoom ? [later, tick] : [tick, ...(isLaterRoom ? [later] : [])]
        order.some((at) => layScratch(context, at))
        return true
    })
}

/** Runs of TRAINING_RUN_ROWS.scratch_run scratches on consecutive cells, each followed by one key on the next cell (M6). */
const layScratchRun = (context: SectionContext) => {
    const rows = runRowsOf('scratch_run')
    const laySingle = createSingleLayer(context)
    for (const run of fixedRuns(context, trainingRunLengthOf('scratch_run'))) {
        run.cells.forEach((cell, row) => {
            const tick = tickOf(context, cell)
            if (row < rows) layScratch(context, tick)
            else laySingle(tick)
        })
    }
}

const HANDS = ['left', 'right'] as const

const otherHand = (hand: (typeof HANDS)[number]) => (hand === 'left' ? 'right' : 'left')

/** Held notes of TRAINING_LN_HOLD_BEATS beats, the hands taking turns, and single keys on the other hand: the heads are part of the target (M6). */
const layLnHold = (context: SectionContext) => {
    const blockTicks = TRAINING_LN_HOLD_BEATS * TRAINING_BEAT_TICKS
    const firstHand = randomItem(context.random, HANDS)
    const lastEnd = context.endTick - context.cellTicksAt(context.endTick - 1)
    const blockCount = Math.ceil((context.endTick - context.startTick) / blockTicks)
    const handOfBlock = (block: number) => (block % 2 === 0 ? firstHand : otherHand(firstHand))
    for (let block = 0; block < blockCount; block += 1) {
        const tick = context.startTick + block * blockTicks
        const endTick = Math.min(tick + blockTicks, lastEnd)
        const lanes = freeLanes(context, TRAINING_HAND_LANES[handOfBlock(block)], tick, endTick)
        if (endTick > tick && lanes.length > 0) context.board.lay(randomItem(context.random, lanes), tick, endTick)
    }
    const layers = { left: createSingleLayer(context, TRAINING_HAND_LANES.left), right: createSingleLayer(context, TRAINING_HAND_LANES.right) }
    layRowsWithDebt(context, (tick) => layers[otherHand(handOfBlock(Math.floor((tick - context.startTick) / blockTicks)))](tick) !== null)
}

/**
 * A long note and, a sixteenth after its end, a hit of the same hand. A row that does not fit is tried on the other hand and with a
 * shorter long note (TRAINING_LN_RELEASE_SHORTENINGS), and carried on when nothing fits, so dense sections keep their rows (M6).
 */
const layLnRelease = (context: SectionContext) => {
    const follow = TRAINING_FOLLOW_SIXTEENTHS * TRAINING_SIXTEENTH_TICKS
    let hand = randomItem(context.random, HANDS)
    const tryLay = (tick: number, onHand: (typeof HANDS)[number], holdTicks: number) => {
        const endTick = tick + holdTicks
        const hitTick = endTick + follow
        if (hitTick >= context.endTick) return false
        const holdLanes = freeLanes(context, TRAINING_HAND_LANES[onHand], tick, endTick)
        if (holdLanes.length === 0) return false
        const holdLane = randomItem(context.random, holdLanes)
        const hitLanes = freeLanes(
            context,
            TRAINING_HAND_LANES[onHand].filter((lane) => lane !== holdLane),
            hitTick,
        )
        if (hitLanes.length === 0) return false
        context.board.lay(holdLane, tick, endTick)
        context.board.lay(randomItem(context.random, hitLanes), hitTick)
        return true
    }
    layRowsWithDebt(context, (tick) => {
        const handsInTurn = hand === HANDS[0] ? HANDS : HANDS.toReversed()
        const tries = TRAINING_LN_RELEASE_SHORTENINGS.flatMap((share) =>
            handsInTurn.map((onHand) => ({ onHand, holdTicks: share * TRAINING_LN_RELEASE_BEATS * TRAINING_BEAT_TICKS })),
        )
        const laid = tries.find((attempt) => tryLay(tick, attempt.onHand, attempt.holdTicks))
        if (laid === undefined) return false
        hand = otherHand(laid.onHand)
        return true
    })
}

const layRest = (context: SectionContext) => {
    let next = 0
    layRowsWithDebt(context, (tick) => {
        const order = context.candidates.map((_, offset) => (next + offset) % context.candidates.length)
        const at = order.find((index) => context.board.isFree(context.candidates[index], tick))
        if (at === undefined) return false
        context.board.lay(context.candidates[at], tick)
        next = (at + 1) % context.candidates.length
        return true
    })
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
    burst: laySingles,
    rest: layRest,
} satisfies Record<TrainingSectionPlan['kind'], (context: SectionContext) => void>

/** `scratch_ratio`: a scratch joins a key row with that chance, where the scratch is free. */
const addScratches = (context: SectionContext, from: number) => {
    const keyTicks = [...new Set(context.board.laid.slice(from).flatMap((note) => (note.lane === TRAINING_SCRATCH_LANE ? [] : [note.tick])))]
    for (const tick of keyTicks.toSorted((a, b) => a - b)) if (context.random() < context.section.scratch_ratio) layScratch(context, tick)
}

/** `ln_ratio`: a key hit becomes a long note with that chance, two cells to a beat long, when it ends inside the section with its lane free. */
const addLongNotes = (context: SectionContext, from: number) => {
    const hits = context.board.laid.flatMap((note, index) =>
        index >= from && note.endTick === null && note.lane !== TRAINING_SCRATCH_LANE ? [index] : [],
    )
    for (const index of hits) {
        if (context.random() >= context.section.ln_ratio) continue
        const { tick, lane } = context.board.laid[index]
        const cellTicks = context.cellTicksAt(tick)
        const shortest = Math.min(TRAINING_LN_MIN_CELLS * cellTicks, TRAINING_BEAT_TICKS)
        const lengths = Math.floor((TRAINING_BEAT_TICKS - shortest) / cellTicks) + 1
        const endTick = tick + shortest + randomIndex(context.random, lengths) * cellTicks
        if (endTick < context.endTick && context.board.isFree(lane, tick + 1, endTick)) context.board.lengthen(index, endTick)
    }
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

type TickClock = ReturnType<typeof createTickClock>

const startTickOf = (section: TrainingSectionPlan) => section.measure_from * TRAINING_MEASURE_TICKS
const endTickOf = (section: TrainingSectionPlan) => (section.measure_from + section.measure_count) * TRAINING_MEASURE_TICKS

/** How much of the density a measure of a section lays: a burst's measures take turns at TRAINING_BURST_HIGH and TRAINING_BURST_LOW. */
const burstFactorOf = (section: TrainingSectionPlan, measureOffset: number) => {
    if (section.kind !== 'burst') return 1
    return measureOffset % 2 === 0 ? TRAINING_BURST_HIGH : TRAINING_BURST_LOW
}

/**
 * The body's density curve (M3-M5): the curve of the bounds' shape over the body, which runs from the first section after the warm-up
 * to the end of the last, before the tail, and the mean μ the curve solves for (and the most μ may be) to bring the body's nps2 p90 to
 * the bounds' target.
 */
const createDensityCurve = (input: TrainingSpecInput, clock: TickClock) => {
    const body = input.sections.filter((section) => !isWarmupSection(section))
    const cutoffUs = TRAINING_LENGTH_US - TRAINING_TAIL_US
    const bodyFromUs = clock(startTickOf(body[0] ?? input.sections[0]))
    const bodyToUs = Math.min(cutoffUs, clock(endTickOf(input.sections[input.sections.length - 1])))
    const noiseSeed = trainingCurveSeedOf(input.seed)
    const curve = trainingCurveOf(trainingCurvePointsOf(input.bounds.shape, noiseSeed), bodyFromUs, bodyToUs)
    const sectionAt = (timeUs: number) =>
        body.find((section) => timeUs < clock(endTickOf(section))) ?? body[body.length - 1] ?? input.sections[input.sections.length - 1]
    const modulationAt = (timeUs: number) => {
        const section = sectionAt(timeUs)
        return burstFactorOf(section, Math.floor((timeUs - clock(startTickOf(section))) / measureUsOf(section.bpm)))
    }
    const solved = solveBodyMean(curve, { ceilingAt: (timeUs) => trainingMaxNpsOf(sectionAt(timeUs)), modulationAt }, input.bounds.p90_target)
    return { curve, noiseSeed, bodyFromUs, mean: solved.mean, meanMax: solved.meanMax }
}

type DensityCurve = ReturnType<typeof createDensityCurve>

/**
 * The density the chart lays at each time with the body mean `mean`: over the body, `mean` times the curve, cut to the ceiling of the
 * section laid there; over the warm-up, TRAINING_WARMUP_DENSITY_RATIO of it (a rest's range for a rest).
 */
const densityPlanOf = (density: DensityCurve, clock: TickClock, mean: number) => {
    const warmupNps = (section: TrainingSectionPlan) => {
        const nps = Math.min(trainingMaxNpsOf(section), Math.max(TRAINING_NPS_FLOOR, mean * TRAINING_WARMUP_DENSITY_RATIO))
        return section.kind === 'rest' ? Math.min(TRAINING_REST_NPS_MAX, Math.max(TRAINING_REST_NPS_MIN, nps)) : nps
    }
    const densityAt = (section: TrainingSectionPlan, tick: number) => {
        if (isWarmupSection(section)) return warmupNps(section)
        return Math.min(trainingMaxNpsOf(section), mean * density.curve.factorAt(clock(tick)))
    }
    return { mean, densityAt }
}

type DensityPlan = ReturnType<typeof densityPlanOf>

/**
 * The cells of a section: each measure on the coarsest of the kind's grids whose cells come at least as fast as the measure's busiest
 * rows (with room for the empty cell after each run of a continuous kind; the finest when none does), each cell carrying the share
 * of a row the density there asks for. A burst's measures take turns at TRAINING_BURST_HIGH and TRAINING_BURST_LOW of the density, so
 * its dense measures read as bursts to the 1-second window.
 */
const cellsOf = (section: TrainingSectionPlan, densities: DensityPlan) => {
    const grids = trainingGridsOf(section.kind)
    const notesPerRow = trainingNotesPerRowOf(section)
    const extra = trainingExtraNpsOf(section)
    const fill = trainingRunFillOf(section.kind)
    const measures = Array.from({ length: section.measure_count }, (_, offset) => {
        const firstTick = (section.measure_from + offset) * TRAINING_MEASURE_TICKS
        const factor = burstFactorOf(section, offset)
        const rowsAt = (tick: number) => (Math.max(0, densities.densityAt(section, tick) - extra) / notesPerRow) * factor
        const busiest = Math.max(...Array.from({ length: TRAINING_MEASURE_TICKS }, (_, tick) => rowsAt(firstTick + tick))) / fill
        const grid = grids.find((candidate) => cellsPerSecondOf(candidate, section.bpm) >= busiest) ?? grids[grids.length - 1]
        const cellTicks = TRAINING_MEASURE_TICKS / grid
        const cells = Array.from({ length: grid }, (_, cell) => firstTick + cell * cellTicks)
        return { grid, cellTicks, cells, shares: cells.map((tick) => Math.min(1, rowsAt(tick) / cellsPerSecondOf(grid, section.bpm))) }
    })
    return {
        grid: Math.max(...measures.map((measure) => measure.grid)),
        cells: measures.flatMap((measure) => measure.cells),
        shares: measures.flatMap((measure) => measure.shares),
        cellTicksAt: (tick: number) =>
            measures[Math.min(measures.length - 1, Math.max(0, Math.floor(tick / TRAINING_MEASURE_TICKS) - section.measure_from))].cellTicks,
    }
}

const laySection = (board: Board, seed: number, section: TrainingSectionPlan, densities: DensityPlan) => {
    const { grid, cells, shares, cellTicksAt } = cellsOf(section, densities)
    const context: SectionContext = {
        section,
        random: sectionRandomOf(seed, section.index),
        board,
        startTick: startTickOf(section),
        endTick: endTickOf(section),
        cells,
        shares,
        cellTicksAt,
        candidates: TRAINING_LANE_CANDIDATES[section.lane_hint],
    }
    const from = board.laid.length
    LAY_KIND[section.kind](context)
    if (section.scratch_ratio > 0 && !isOneOf(TRAINING_SCRATCH_KINDS, section.kind)) addScratches(context, from)
    if (section.ln_ratio > 0 && !isOneOf(TRAINING_LN_KINDS, section.kind)) addLongNotes(context, from)
    return grid
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

/** One layout of a spec with the body mean `mean`: the notes kept before the tail, their times and the body's nps2 p90. */
const layChart = (input: TrainingSpecInput, clock: TickClock, density: DensityCurve, mean: number) => {
    const densities = densityPlanOf(density, clock, mean)
    const board = createBoard()
    const grids = input.sections.map((section) => laySection(board, input.seed, section, densities))
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
    const placed = notes.map((note) => ({ timeUs: timeline.timeAt(note.measure + note.num / note.den), lane: note.lane, kind: note.kind }))
    const local = localDensitiesOf(placed)
    const bodyDensities = placed.flatMap((note, at) => (note.kind !== 'ln_end' && note.timeUs >= density.bodyFromUs ? [local[at].nps2] : []))
    return { densities, grids, keptBoard, notes, timeline, bodyP90: nearestRankOf(bodyDensities, TRAINING_BODY_PERCENTILE) }
}

/**
 * Lays the notes of a spec (plan 3.6, 2026-11-04-training-difficulty M4-M10) and fills what the layout decides: each section's times,
 * finest grid, target (the curve's mean over it), notes, actual NPS and rows a second, the body mean and the curve's noise seed in the
 * bounds, and the chart's play level (the body's nps2 p90, rounded). The body mean starts at the one the curve solves for; while the
 * laid body's nps2 p90 is off the target by more than TRAINING_P90_TOLERANCE, it is scaled by target / p90 (up to TRAINING_MEAN_PASSES
 * layouts, within TRAINING_MEAN_REFINE_RATIO of the solved mean and never past the most the curve allows), and the layout nearest the
 * target is kept. The same spec gives the same notes: every
 * section draws from its own generator seeded by the spec's seed, the curve from its own, and the fields the synthesiser fills are not
 * read. No note is left at or after the tail, and a chart with no key 6 or 7 gets one.
 */
export const synthesizeTraining = (input: TrainingSpecInput) => {
    const clock = createTickClock(input)
    const density = createDensityCurve(input, clock)
    const target = input.bounds.p90_target
    const offBy = (layout: ReturnType<typeof layChart>) => Math.abs(layout.bodyP90 - target) / target
    let layout = layChart(input, clock, density, density.mean)
    let best = layout
    const lowest = density.mean / TRAINING_MEAN_REFINE_RATIO
    const highest = Math.min(density.meanMax, density.mean * TRAINING_MEAN_REFINE_RATIO)
    for (let pass = 1; pass < TRAINING_MEAN_PASSES && offBy(best) > TRAINING_P90_TOLERANCE && layout.bodyP90 > 0; pass += 1) {
        layout = layChart(input, clock, density, Math.min(highest, Math.max(lowest, (layout.densities.mean * target) / layout.bodyP90)))
        if (offBy(layout) < offBy(best)) best = layout
    }
    const { densities, grids, keptBoard, notes, timeline, bodyP90 } = best
    const sections = input.sections.map((section, index): TrainingSection => {
        const startTick = startTickOf(section)
        const endTick = endTickOf(section)
        const inSection = keptBoard.laid.filter((note) => note.tick >= startTick && note.tick < endTick)
        const rows = new Set(inSection.map((note) => note.tick)).size
        const fromUs = timeline.timeAt(section.measure_from)
        const toUs = timeline.timeAt(section.measure_from + section.measure_count)
        const perSecond = (count: number) => (toUs > fromUs ? roundNps((count * MICROS_PER_SECOND) / (toUs - fromUs)) : 0)
        const targets = Array.from({ length: endTick - startTick }, (_, offset) => densities.densityAt(section, startTick + offset))
        return {
            index: section.index,
            focus_key: section.focus_key,
            kind: section.kind,
            lane_hint: section.lane_hint,
            chord_size: section.chord_size,
            scratch_ratio: section.scratch_ratio,
            ln_ratio: section.ln_ratio,
            bpm: section.bpm,
            target_nps: roundNps(targets.reduce((sum, value) => sum + value, 0) / targets.length),
            measure_from: section.measure_from,
            measure_count: section.measure_count,
            from_us: fromUs,
            to_us: toUs,
            grid: grids[index],
            actual_nps: perSecond(inSection.length),
            actual_rps: perSecond(rows),
            notes: inSection.length,
            intent: section.intent,
        }
    })
    const spec = {
        generator_version: input.generator_version,
        seed: input.seed,
        base_bpm: input.base_bpm,
        title: input.title,
        subtitle: input.subtitle,
        play_level: Math.min(TRAINING_PLAY_LEVEL_MAX, Math.max(TRAINING_PLAY_LEVEL_MIN, Math.round(bodyP90))),
        bounds: {
            limit_nps: input.bounds.limit_nps,
            p90_min: input.bounds.p90_min,
            p90_max: input.bounds.p90_max,
            level: input.bounds.level,
            p90_target: input.bounds.p90_target,
            shape: input.bounds.shape,
            mean_nps: roundNps(densities.mean),
            noise_seed: density.noiseSeed,
        },
        sections,
        corrections: input.corrections,
    }
    return { spec, notes }
}
