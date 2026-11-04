import { createHash } from 'node:crypto'
import { z } from 'zod'
import { TRAINING_INTENSITY_TITLE, TRAINING_LANE_HINTS, TRAINING_LIMIT, TRAINING_PATTERN_KINDS, TRAINING_SHAPES } from '@server/dto/training'
import type { AiTrainingPlanOutput, TrainingIntensity, TrainingLaneHint, TrainingPatternKind } from '@server/dto/training'
import { createAppError } from '@server/lib/error'
import {
    BEATS_PER_MEASURE,
    GENERATOR_VERSION,
    MICROS_PER_MEASURE_AT_ONE_BPM,
    MICROS_PER_SECOND,
    SECONDS_PER_MINUTE,
    TRAINING_BPM_MAX,
    TRAINING_BPM_MIN,
    TRAINING_BPM_RATIOS,
    TRAINING_BPM_STEP,
    TRAINING_BURST_HIGH,
    TRAINING_CHORD_FILL_KINDS,
    TRAINING_CHORD_FILL_ROW_NOTES_MAX,
    TRAINING_CHORD_KIND_MIN_SIZE,
    TRAINING_CHORD_KINDS,
    TRAINING_CHORD_SIZE_MAX,
    TRAINING_CHORD_SIZE_MIN,
    TRAINING_DEFAULT_LIMIT_NPS,
    TRAINING_EFFECTIVE_BPM_MAX,
    TRAINING_EFFECTIVE_BPM_MIN,
    TRAINING_FIRST_NOTE_MEASURE,
    TRAINING_GRIDS,
    TRAINING_ID_PREFIX,
    TRAINING_INTENSITY_RANGE,
    TRAINING_LENGTH_US,
    TRAINING_LN_HOLD_BEATS,
    TRAINING_LN_KINDS,
    TRAINING_LN_RELEASE_GRIDS,
    TRAINING_LN_RELEASE_ROW_NOTES,
    TRAINING_MIX_RATIO_MAX,
    TRAINING_NEUTRAL_BPM_RATIO,
    TRAINING_NPS_CEIL,
    TRAINING_NPS_DECIMALS,
    TRAINING_NPS_FLOOR,
    TRAINING_PEAK_OVER_P90,
    TRAINING_REST_GRID,
    TRAINING_REST_NPS_MAX,
    TRAINING_RUN_GAP_KINDS,
    TRAINING_RUN_GRIDS,
    TRAINING_RUN_ROWS,
    TRAINING_SCRATCH_KINDS,
    TRAINING_SCRATCH_MIXED_SHARE,
    TRAINING_SECTION_MIN_MEASURES,
    TRAINING_SHORT_ID_LENGTH,
    TRAINING_TAIL_US,
    TRAINING_TITLE_PREFIX,
    TRAINING_TITLE_SEPARATOR,
    TRAINING_WARMUP_KEY,
    TRAINING_WARMUP_MAX_SHARE,
    TRAINING_WEIGHT_MAX,
    TRAINING_WEIGHT_MIN,
    trainingKindsOfKey,
    type TrainingContinuousKind,
} from '@server/service/domain/training/training-constants'

export const TRAINING_CORRECTION_FIELDS = [
    'focus_key',
    'kind',
    'base_bpm',
    'bpm_ratio',
    'weight',
    'measures',
    'target_nps',
    'chord_size',
    'scratch_ratio',
    'ln_ratio',
    'level',
    'chord_fill',
] as const
export type TrainingCorrectionField = (typeof TRAINING_CORRECTION_FIELDS)[number]

const correctionValueSchema = z.union([z.string(), z.number()]).nullable()

/** One value the server changed from the provider's plan. `section` is the plan's own section number (from 0), null for the whole chart. */
export const trainingCorrectionSchema = z.object({
    section: z.number().int().nonnegative().nullable(),
    field: z.enum(TRAINING_CORRECTION_FIELDS),
    from: correctionValueSchema,
    to: correctionValueSchema,
})

/** A section as the corrected plan lays it out, before the synthesiser places its notes. */
export const trainingSectionPlanSchema = z.object({
    index: z.number().int().nonnegative(),
    focus_key: z.string(),
    kind: z.enum(TRAINING_PATTERN_KINDS),
    lane_hint: z.enum(TRAINING_LANE_HINTS),
    chord_size: z.number().int().min(TRAINING_CHORD_SIZE_MIN).max(TRAINING_CHORD_SIZE_MAX),
    scratch_ratio: z.number().min(0).max(TRAINING_MIX_RATIO_MAX),
    ln_ratio: z.number().min(0).max(TRAINING_MIX_RATIO_MAX),
    bpm: z.number().positive(),
    measure_from: z.number().int().min(TRAINING_FIRST_NOTE_MEASURE),
    measure_count: z.number().int().min(TRAINING_SECTION_MIN_MEASURES),
    intent: z.string(),
})

/**
 * A section of a stored spec: its plan and what the synthesiser laid (times in microseconds, `notes` the playable notes, `target_nps`
 * the density curve's mean over the section, `grid` the finest grid it used). Generator 1 has no `actual_rps` (rows a second).
 */
export const trainingSectionSchema = trainingSectionPlanSchema.extend({
    target_nps: z.number().positive(),
    from_us: z.number().int().nonnegative(),
    to_us: z.number().int().nonnegative(),
    grid: z.number().int().positive(),
    actual_nps: z.number().nonnegative(),
    actual_rps: z.number().nonnegative().optional(),
    notes: z.number().int().nonnegative(),
})

/**
 * The density a corrected plan is held to (plan 2026-11-04-training-difficulty M3, M10): the player's limit L, the intensity's range
 * of the body's nps2 p90, the plan's level within it and the p90 it gives, and the curve's shape. The synthesiser fills the body mean
 * μ it solved for and the seed of the curve's noise, so a stored chart tells how its densities came about.
 */
export const trainingSpecBoundsSchema = z.object({
    limit_nps: z.number().positive(),
    p90_min: z.number().positive(),
    p90_max: z.number().positive(),
    level: z.number().min(0).max(1),
    p90_target: z.number().positive(),
    shape: z.enum(TRAINING_SHAPES),
    mean_nps: z.number().nonnegative(),
    noise_seed: z.number().int().nonnegative(),
})

/** The stored spec of a training chart (`training_chart.spec`, plan 3.5): the same spec gives the same chart bytes. Generator 1 has no bounds. */
export const trainingSpecSchema = z.object({
    generator_version: z.number().int().positive(),
    seed: z.number().int().nonnegative(),
    base_bpm: z.number().int().positive(),
    title: z.string(),
    subtitle: z.string(),
    play_level: z.number().int(),
    bounds: trainingSpecBoundsSchema.optional(),
    sections: z.array(trainingSectionSchema).min(1),
    corrections: z.array(trainingCorrectionSchema),
})

export type TrainingCorrection = z.infer<typeof trainingCorrectionSchema>
export type TrainingSectionPlan = z.infer<typeof trainingSectionPlanSchema>
export type TrainingSection = z.infer<typeof trainingSectionSchema>
export type TrainingSpec = z.infer<typeof trainingSpecSchema>
export type TrainingSpecBounds = z.infer<typeof trainingSpecBoundsSchema>

/** The bounds a corrected plan carries: what the synthesiser reads of them. */
export type TrainingPlanBounds = Omit<TrainingSpecBounds, 'mean_nps' | 'noise_seed'>

/** What the synthesiser reads: the corrected plan, its bounds, the seed and the chart's own text. The fields it fills are not read. */
export type TrainingSpecInput = Omit<TrainingSpec, 'play_level' | 'sections' | 'bounds'> & {
    sections: TrainingSectionPlan[]
    bounds: TrainingPlanBounds
}

export type CorrectedTrainingPlan = Pick<TrainingSpecInput, 'base_bpm' | 'sections' | 'bounds'>

/** The density figures of a profile the range is built from (plan 3.2 `density`). */
export type TrainingDensity = { limit_nps_ex: number | null; max_seen_nps: number | null }

/** The ranges a plan is held to (plan 3.2 `bounds`): the body's nps2 p90 range of the intensity, the tempo, section and length limits. */
export type TrainingBounds = ReturnType<typeof trainingBoundsOf>

/** What a plan may name (plan 3.2 `allowed`). */
export type TrainingAllowed = { kinds: readonly TrainingPatternKind[]; lane_hints: readonly TrainingLaneHint[]; keys: readonly string[] }

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

const NPS_SCALE = 10 ** TRAINING_NPS_DECIMALS

/** An NPS figure kept to TRAINING_NPS_DECIMALS places. */
export const roundNps = (value: number) => Math.round(value * NPS_SCALE) / NPS_SCALE

/** The length of one measure at `bpm`, in microseconds (not truncated: for comparing lengths only). */
export const measureUsOf = (bpm: number) => MICROS_PER_MEASURE_AT_ONE_BPM / bpm

const withinChart = (value: number) => roundNps(clamp(value, TRAINING_NPS_FLOOR, TRAINING_NPS_CEIL))

/**
 * The range of an intensity for a player's density (M1, M2): the body's nps2 p90 between the intensity's shares of the limit L, with
 * the tempo, section and length limits of every chart. L is the EX limit, else the highest density seen, else TRAINING_DEFAULT_LIMIT_NPS.
 */
export const trainingBoundsOf = (density: TrainingDensity, intensity: TrainingIntensity) => {
    const limit = density.limit_nps_ex ?? density.max_seen_nps ?? TRAINING_DEFAULT_LIMIT_NPS
    const range = TRAINING_INTENSITY_RANGE[intensity]
    const p90Min = withinChart(limit * range.min)
    return {
        limit_nps: withinChart(limit),
        p90_min: p90Min,
        p90_max: Math.max(p90Min, withinChart(limit * range.max)),
        bpm_min: TRAINING_BPM_MIN,
        bpm_max: TRAINING_BPM_MAX,
        sections_min: TRAINING_LIMIT.SECTIONS_MIN,
        sections_max: TRAINING_LIMIT.SECTIONS_MAX,
        length_s: TRAINING_LENGTH_US / MICROS_PER_SECOND,
        warmup_max_s: (TRAINING_LENGTH_US * TRAINING_WARMUP_MAX_SHARE) / MICROS_PER_SECOND,
    }
}

/** Whether a section is the warm-up, which the body's density curve and statistics leave out. */
export const isWarmupSection = (section: { focus_key: string }) => section.focus_key === TRAINING_WARMUP_KEY

/** Whether `value` is one of the kinds of `list`. */
export const isOneOf = <T extends string>(list: readonly T[], value: string) => (list as readonly string[]).includes(value)

const NOTES_PER_ROW_OF_KIND: Partial<Record<TrainingPatternKind, number>> = {
    scratch_solo: 1,
    scratch_mixed: 1 + TRAINING_SCRATCH_MIXED_SHARE,
    scratch_run: 1,
    ln_release: TRAINING_LN_RELEASE_ROW_NOTES,
}

type RowShape = { kind: TrainingPatternKind; chord_size: number; scratch_ratio: number }

/**
 * The playable notes one row of a section lays on average (plan 3.6, M6), so `target_nps / notesPerRow` rows a second meet the target:
 * a row of `scratch_solo` or `scratch_run` is one scratch or one key.
 */
export const trainingNotesPerRowOf = (section: RowShape) => {
    if (isOneOf(TRAINING_CHORD_KINDS, section.kind)) return section.chord_size
    return NOTES_PER_ROW_OF_KIND[section.kind] ?? 1 + section.scratch_ratio
}

/** The notes a second a section lays besides its rows: `ln_hold`'s held notes, a head every TRAINING_LN_HOLD_BEATS beats (M6). */
export const trainingExtraNpsOf = (section: { kind: TrainingPatternKind; bpm: number }) =>
    section.kind === 'ln_hold' ? section.bpm / SECONDS_PER_MINUTE / TRAINING_LN_HOLD_BEATS : 0

/** Whether a kind lays runs of rows on consecutive cells (plan 3.6), which the classifier needs to see a trill, a stair or a jack. */
export const isContinuousKind = (kind: TrainingPatternKind): kind is TrainingContinuousKind => Object.hasOwn(TRAINING_RUN_ROWS, kind)

/** The rows of one run of a continuous kind: a `scratch_run` run is its scratches and the key after them. */
export const trainingRunLengthOf = (kind: TrainingContinuousKind) => TRAINING_RUN_ROWS[kind] + (kind === 'scratch_run' ? 1 : 0)

/** The share of its cells a kind can fill: a kind of TRAINING_RUN_GAP_KINDS leaves an empty cell after each run (M6). */
export const trainingRunFillOf = (kind: TrainingPatternKind) => {
    if (!isContinuousKind(kind) || !isOneOf(TRAINING_RUN_GAP_KINDS, kind)) return 1
    const length = trainingRunLengthOf(kind)
    return length / (length + 1)
}

/** The grids a section of a kind may be laid on, coarsest first. */
export const trainingGridsOf = (kind: TrainingPatternKind): readonly number[] => {
    if (kind === 'rest') return [TRAINING_REST_GRID]
    if (kind === 'ln_release') return TRAINING_LN_RELEASE_GRIDS
    return isContinuousKind(kind) ? TRAINING_RUN_GRIDS : TRAINING_GRIDS
}

/** How much faster than its average a section's busiest measure is: a burst's dense measure must fit its grid. */
export const trainingPeakFactorOf = (kind: TrainingPatternKind) => (kind === 'burst' ? TRAINING_BURST_HIGH : 1)

/** Cells a second on a grid of `grid` cells a measure at `bpm`. */
export const cellsPerSecondOf = (grid: number, bpm: number) => (grid * bpm) / (BEATS_PER_MEASURE * SECONDS_PER_MINUTE)

/** The most NPS a section can lay: a row on every cell of its finest grid in its busiest measure, a rest no more than a rest's most. */
export const trainingMaxNpsOf = (section: RowShape & { bpm: number }) => {
    if (section.kind === 'rest') return TRAINING_REST_NPS_MAX
    const grids = trainingGridsOf(section.kind)
    const cells = cellsPerSecondOf(grids[grids.length - 1], section.bpm) * trainingRunFillOf(section.kind)
    const rows = (cells * trainingNotesPerRowOf(section)) / trainingPeakFactorOf(section.kind)
    return Math.min(TRAINING_NPS_CEIL, rows + trainingExtraNpsOf(section))
}

/**
 * The most NPS a section can lay once its rows are turned into chords where the density asks for more (TRAINING_CHORD_FILL_KINDS):
 * TRAINING_CHORD_FILL_ROW_NOTES_MAX keys a row on every cell of its finest grid. Other kinds lay no more than `trainingMaxNpsOf`.
 */
export const trainingFilledMaxNpsOf = (section: RowShape & { bpm: number }) => {
    const rowsMaxNps = trainingMaxNpsOf(section)
    if (!isOneOf(TRAINING_CHORD_FILL_KINDS, section.kind)) return rowsMaxNps
    const filledNotesPerRow = TRAINING_CHORD_FILL_ROW_NOTES_MAX + section.scratch_ratio
    return Math.min(TRAINING_NPS_CEIL, (rowsMaxNps * filledNotesPerRow) / trainingNotesPerRowOf(section))
}

/** The base tempo held to the chart's range and rounded to a multiple of TRAINING_BPM_STEP. */
export const correctBaseBpm = (bpm: number) => Math.round(clamp(bpm, TRAINING_BPM_MIN, TRAINING_BPM_MAX) / TRAINING_BPM_STEP) * TRAINING_BPM_STEP

/** The allowed rate nearest `ratio` (the smaller on a tie), stepped toward 1 while the section's tempo is outside the effective range. */
export const correctBpmRatio = (ratio: number, baseBpm: number) => {
    const nearest = TRAINING_BPM_RATIOS.reduce((best, candidate) => (Math.abs(candidate - ratio) < Math.abs(best - ratio) ? candidate : best))
    let at = TRAINING_BPM_RATIOS.indexOf(nearest)
    const neutral = TRAINING_BPM_RATIOS.indexOf(TRAINING_NEUTRAL_BPM_RATIO)
    const isWithin = (index: number) => {
        const bpm = baseBpm * TRAINING_BPM_RATIOS[index]
        return bpm >= TRAINING_EFFECTIVE_BPM_MIN && bpm <= TRAINING_EFFECTIVE_BPM_MAX
    }
    while (at !== neutral && !isWithin(at)) at += at < neutral ? 1 : -1
    return TRAINING_BPM_RATIOS[at]
}

/**
 * How many measures each section gets (plan 3.5 `distributeMeasures`): the time after the count-in and the tail is shared by weight,
 * then measures are taken from the longest section while the sections run over that time and given to the heaviest section that
 * still fits while any does. A section with `max` gets no more measures than that (the warm-up, M5). The result keeps
 * `A − min M(bpm_i) < Σ m_i × M(bpm_i) ≤ A` while a section without `max` can grow, every section one measure or more.
 */
export const distributeMeasures = (sections: readonly { weight: number; bpm: number; max?: number }[], baseBpm: number) => {
    const available = TRAINING_LENGTH_US - measureUsOf(baseBpm) - TRAINING_TAIL_US
    const totalWeight = sections.reduce((sum, section) => sum + section.weight, 0)
    const lengths = sections.map((section) => measureUsOf(section.bpm))
    const capOf = (index: number) => sections[index].max ?? Number.POSITIVE_INFINITY
    const rounded = sections.map((section, index) =>
        Math.max(TRAINING_SECTION_MIN_MEASURES, Math.round((available * section.weight) / totalWeight / lengths[index])),
    )
    const measures = rounded.map((count, index) => Math.min(capOf(index), count))
    const totalOf = (counts: readonly number[]) => counts.reduce((sum, count, index) => sum + count * lengths[index], 0)
    while (totalOf(measures) > available) {
        const shrinkable = measures.flatMap((count, index) => (count > TRAINING_SECTION_MIN_MEASURES ? [index] : []))
        if (shrinkable.length === 0) break
        const longest = shrinkable.reduce((best, index) => (measures[index] >= measures[best] ? index : best))
        measures[longest] -= 1
    }
    const byWeight = sections.map((_, index) => index).toSorted((a, b) => sections[b].weight - sections[a].weight || a - b)
    const grownAt = (index: number) => measures.map((count, at) => (at === index ? count + 1 : count))
    let added = true
    while (added) {
        const fitting = byWeight.find((index) => measures[index] < capOf(index) && totalOf(grownAt(index)) <= available)
        added = fitting !== undefined
        if (fitting !== undefined) measures[fitting] += 1
    }
    return { rounded, measures }
}

/** The most measures a warm-up at `bpm` may take: TRAINING_WARMUP_MAX_SHARE of the chart, one measure at least. */
export const warmupMaxMeasuresOf = (bpm: number) =>
    Math.max(TRAINING_SECTION_MIN_MEASURES, Math.floor((TRAINING_LENGTH_US * TRAINING_WARMUP_MAX_SHARE) / measureUsOf(bpm)))

type KeptSection = { section: AiTrainingPlanOutput['sections'][number]; original: number }

/** A kept section as it is laid at `baseBpm`: its kind turned into one that trains its key and its numbers held to their ranges. */
const shapeSectionAt = ({ section }: KeptSection, baseBpm: number) => {
    const kinds = trainingKindsOfKey(section.focus_key)
    const kind = kinds.length === 0 || kinds.includes(section.kind) ? section.kind : kinds[0]
    const ratio = correctBpmRatio(section.bpm_ratio, baseBpm)
    const isChordKind = isOneOf(TRAINING_CHORD_KINDS, kind)
    return {
        focus_key: section.focus_key,
        kind,
        ratio,
        weight: clamp(section.weight, TRAINING_WEIGHT_MIN, TRAINING_WEIGHT_MAX),
        chord_size: isChordKind
            ? clamp(Math.round(section.chord_size), TRAINING_CHORD_KIND_MIN_SIZE, TRAINING_CHORD_SIZE_MAX)
            : TRAINING_CHORD_SIZE_MIN,
        scratch_ratio: isOneOf(TRAINING_SCRATCH_KINDS, kind) ? 0 : clamp(section.scratch_ratio, 0, TRAINING_MIX_RATIO_MAX),
        ln_ratio: isOneOf(TRAINING_LN_KINDS, kind) ? 0 : clamp(section.ln_ratio, 0, TRAINING_MIX_RATIO_MAX),
        bpm: baseBpm * ratio,
    }
}

/** The base tempos from `from` up to TRAINING_BPM_MAX, a TRAINING_BPM_STEP apart. */
const baseTemposFrom = (from: number) =>
    Array.from({ length: (TRAINING_BPM_MAX - from) / TRAINING_BPM_STEP + 1 }, (_, step) => from + step * TRAINING_BPM_STEP)

/**
 * Turns a provider's plan into the plan the synthesiser lays out (plan 3.5, 2026-11-04-training-difficulty M2, M5, M7). Sections whose
 * `focus_key` is not allowed are dropped, and so is a warm-up that is not the first section kept; a kind that does not train its key
 * becomes the key's first kind, the level is held to [0, 1], every number to its range, and the warm-up to TRAINING_WARMUP_MAX_SHARE
 * of the chart; each change is a correction. The body's nps2 p90 is the range's point at the level. When a section's finest grid
 * cannot lay the busiest density that p90 brings (TRAINING_PEAK_OVER_P90), the base tempo is raised a step at a time up to
 * TRAINING_BPM_MAX. It runs again on every request, cached plan or not, so the corrections are the same either way. Fewer than
 * `TRAINING_LIMIT.SECTIONS_MIN` sections left is refused (`refineTrainingPlanOutput` has refused such a plan already).
 */
export const correctTrainingPlan = (plan: AiTrainingPlanOutput, context: { allowed: TrainingAllowed; bounds: TrainingBounds }) => {
    const corrections: TrainingCorrection[] = []
    const record = (section: number | null, field: TrainingCorrectionField, from: string | number, to: string | number) => {
        if (from !== to) corrections.push({ section, field, from, to })
    }
    const kept = plan.sections.reduce<KeptSection[]>((list, section, original) => {
        const isLateWarmup = isWarmupSection(section) && list.length > 0
        if (context.allowed.keys.includes(section.focus_key) && !isLateWarmup) return [...list, { section, original }]
        corrections.push({ section: original, field: 'focus_key', from: section.focus_key, to: null })
        return list
    }, [])
    if (kept.length < TRAINING_LIMIT.SECTIONS_MIN) throw createAppError('AI_OUTPUT_INVALID', { sections: kept.length })

    const level = clamp(plan.level, 0, 1)
    record(null, 'level', plan.level, level)
    const { limit_nps, p90_min, p90_max } = context.bounds
    const p90Target = roundNps(p90_min + level * (p90_max - p90_min))
    const peakNps = p90Target * TRAINING_PEAK_OVER_P90
    const fitsPeak = (baseBpm: number) =>
        kept.every((entry) => isWarmupSection(entry.section) || trainingMaxNpsOf(shapeSectionAt(entry, baseBpm)) >= peakNps)
    const baseBpm = baseTemposFrom(correctBaseBpm(plan.base_bpm)).find(fitsPeak) ?? TRAINING_BPM_MAX
    record(null, 'base_bpm', plan.base_bpm, baseBpm)

    const laid = kept.map((entry) => {
        const { section, original } = entry
        const shaped = shapeSectionAt(entry, baseBpm)
        record(original, 'kind', section.kind, shaped.kind)
        record(original, 'bpm_ratio', section.bpm_ratio, shaped.ratio)
        record(original, 'weight', section.weight, shaped.weight)
        record(original, 'chord_size', section.chord_size, shaped.chord_size)
        record(original, 'scratch_ratio', section.scratch_ratio, shaped.scratch_ratio)
        record(original, 'ln_ratio', section.ln_ratio, shaped.ln_ratio)
        return {
            original,
            weight: shaped.weight,
            plan: {
                focus_key: shaped.focus_key,
                kind: shaped.kind,
                lane_hint: section.lane_hint,
                chord_size: shaped.chord_size,
                scratch_ratio: shaped.scratch_ratio,
                ln_ratio: shaped.ln_ratio,
                bpm: shaped.bpm,
                intent: section.intent,
            },
        }
    })

    const { rounded, measures } = distributeMeasures(
        laid.map((section) => ({
            weight: section.weight,
            bpm: section.plan.bpm,
            max: isWarmupSection(section.plan) ? warmupMaxMeasuresOf(section.plan.bpm) : undefined,
        })),
        baseBpm,
    )
    let measureFrom = TRAINING_FIRST_NOTE_MEASURE
    const sections = laid.map((section, index): TrainingSectionPlan => {
        record(section.original, 'measures', rounded[index], measures[index])
        const { intent, ...rest } = section.plan
        const placed = { index, ...rest, measure_from: measureFrom, measure_count: measures[index], intent }
        measureFrom += measures[index]
        return placed
    })
    const bounds = { limit_nps, p90_min, p90_max, level, p90_target: p90Target, shape: plan.shape } satisfies TrainingPlanBounds
    return { spec: { base_bpm: baseBpm, sections, bounds } satisfies CorrectedTrainingPlan, corrections }
}

const canonicalJsonOf = (value: unknown): string => {
    if (Array.isArray(value)) return `[${value.map(canonicalJsonOf).join(',')}]`
    if (value === null || typeof value !== 'object') return JSON.stringify(value)
    const entries = Object.entries(value).toSorted(([left], [right]) => (left < right ? -1 : 1))
    return `{${entries.map(([key, entry]) => `${JSON.stringify(key)}:${canonicalJsonOf(entry)}`).join(',')}}`
}

const SEED_HEX_LENGTH = 8
const HEX_RADIX = 16

/** The seed of a new chart: the first 32 bits of SHA-256 over the user, the moment and the corrected plan (plan 3.5). */
export const trainingSeedOf = (params: { userId: string; createdAt: number; plan: CorrectedTrainingPlan }) =>
    Number.parseInt(
        createHash('sha256')
            .update(`${params.userId}:${params.createdAt}:${canonicalJsonOf(params.plan)}`)
            .digest('hex')
            .slice(0, SEED_HEX_LENGTH),
        HEX_RADIX,
    )

/** The hex characters of an id the subtitle and the file names carry: those after `tc_`. */
export const trainingShortIdOf = (id: string) => id.slice(TRAINING_ID_PREFIX.length, TRAINING_ID_PREFIX.length + TRAINING_SHORT_ID_LENGTH)

const ISO_DATE_LENGTH = 10

/** The public `#TITLE`: a fixed shape with the UTC day it was made and the intensity, naming no weakness (plan 3.7). */
export const trainingTitleOf = (createdAt: number, intensity: TrainingIntensity) =>
    `${TRAINING_TITLE_PREFIX}${new Date(createdAt).toISOString().slice(0, ISO_DATE_LENGTH)}${TRAINING_TITLE_SEPARATOR}${TRAINING_INTENSITY_TITLE[intensity]}`

/** The `#SUBTITLE`: the short id in brackets, which makes every chart's bytes and md5 its own. */
export const trainingSubtitleOf = (id: string) => `[${trainingShortIdOf(id)}]`

/** The synthesiser's input for a corrected plan of chart `id`, made at `createdAt` by `userId`. */
export const trainingSpecInputOf = (params: {
    id: string
    userId: string
    createdAt: number
    intensity: TrainingIntensity
    plan: CorrectedTrainingPlan
    corrections: TrainingCorrection[]
}) =>
    ({
        generator_version: GENERATOR_VERSION,
        seed: trainingSeedOf(params),
        base_bpm: params.plan.base_bpm,
        title: trainingTitleOf(params.createdAt, params.intensity),
        subtitle: trainingSubtitleOf(params.id),
        bounds: params.plan.bounds,
        sections: params.plan.sections,
        corrections: params.corrections,
    }) satisfies TrainingSpecInput
