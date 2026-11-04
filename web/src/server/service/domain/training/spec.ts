import { createHash } from 'node:crypto'
import { z } from 'zod'
import { TRAINING_INTENSITY_TITLE, TRAINING_LANE_HINTS, TRAINING_LIMIT, TRAINING_PATTERN_KINDS } from '@server/dto/training'
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
    TRAINING_CHORD_KIND_MIN_SIZE,
    TRAINING_CHORD_KINDS,
    TRAINING_CHORD_SIZE_MAX,
    TRAINING_CHORD_SIZE_MIN,
    TRAINING_COMFORT_FALLBACK_RATIO,
    TRAINING_DEFAULT_LIMIT_NPS,
    TRAINING_EDGE_KEYS,
    TRAINING_EFFECTIVE_BPM_MAX,
    TRAINING_EFFECTIVE_BPM_MIN,
    TRAINING_FIRST_NOTE_MEASURE,
    TRAINING_ID_PREFIX,
    TRAINING_INTENSITY_RANGE,
    TRAINING_LENGTH_US,
    TRAINING_LN_KINDS,
    TRAINING_MIX_RATIO_MAX,
    TRAINING_NEUTRAL_BPM_RATIO,
    TRAINING_NPS_CEIL,
    TRAINING_NPS_DECIMALS,
    TRAINING_NPS_FLOOR,
    TRAINING_BURST_HIGH,
    TRAINING_GRIDS,
    TRAINING_LN_RELEASE_ROW_NOTES,
    TRAINING_REST_GRID,
    TRAINING_REST_NPS_MAX,
    TRAINING_REST_NPS_MIN,
    TRAINING_RUN_GRIDS,
    TRAINING_RUN_ROWS,
    TRAINING_SCRATCH_KINDS,
    TRAINING_SCRATCH_MIXED_SHARE,
    TRAINING_SCRATCH_SOLO_KEY_SHARE,
    TRAINING_SECTION_MIN_MEASURES,
    TRAINING_SHORT_ID_LENGTH,
    TRAINING_TAIL_US,
    TRAINING_TITLE_PREFIX,
    TRAINING_TITLE_SEPARATOR,
    TRAINING_WEIGHT_MAX,
    TRAINING_WEIGHT_MIN,
    trainingKindsOfKey,
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
    target_nps: z.number().positive(),
    measure_from: z.number().int().min(TRAINING_FIRST_NOTE_MEASURE),
    measure_count: z.number().int().min(TRAINING_SECTION_MIN_MEASURES),
    intent: z.string(),
})

/** A section of a stored spec: its plan and what the synthesiser laid (times in microseconds, `notes` the playable notes). */
export const trainingSectionSchema = trainingSectionPlanSchema.extend({
    from_us: z.number().int().nonnegative(),
    to_us: z.number().int().nonnegative(),
    grid: z.number().int().positive(),
    actual_nps: z.number().nonnegative(),
    notes: z.number().int().nonnegative(),
})

/** The stored spec of a training chart (`training_chart.spec`, plan 3.5): the same spec gives the same chart bytes. */
export const trainingSpecSchema = z.object({
    generator_version: z.number().int().positive(),
    seed: z.number().int().nonnegative(),
    base_bpm: z.number().int().positive(),
    title: z.string(),
    subtitle: z.string(),
    play_level: z.number().int(),
    sections: z.array(trainingSectionSchema).min(1),
    corrections: z.array(trainingCorrectionSchema),
})

export type TrainingCorrection = z.infer<typeof trainingCorrectionSchema>
export type TrainingSectionPlan = z.infer<typeof trainingSectionPlanSchema>
export type TrainingSection = z.infer<typeof trainingSectionSchema>
export type TrainingSpec = z.infer<typeof trainingSpecSchema>

/** What the synthesiser reads: the corrected plan, the seed and the chart's own text. The fields it fills are not read. */
export type TrainingSpecInput = Omit<TrainingSpec, 'play_level' | 'sections'> & { sections: TrainingSectionPlan[] }

export type CorrectedTrainingPlan = Pick<TrainingSpecInput, 'base_bpm' | 'sections'>

/** The density figures of a profile the NPS range is built from (plan 3.2 `density`). */
export type TrainingDensity = { comfort_nps: number | null; limit_nps_ex: number | null; max_seen_nps: number | null }

/** The ranges a plan is held to (plan 3.2 `bounds`), with the comfort density the warm-up and cool-down sections stay under. */
export type TrainingBounds = ReturnType<typeof trainingBoundsOf>

/** What a plan may name (plan 3.2 `allowed`). */
export type TrainingAllowed = { kinds: readonly TrainingPatternKind[]; lane_hints: readonly TrainingLaneHint[]; keys: readonly string[] }

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

const NPS_SCALE = 10 ** TRAINING_NPS_DECIMALS

/** An NPS figure kept to TRAINING_NPS_DECIMALS places. */
export const roundNps = (value: number) => Math.round(value * NPS_SCALE) / NPS_SCALE

/** The length of one measure at `bpm`, in microseconds (not truncated: for comparing lengths only). */
export const measureUsOf = (bpm: number) => MICROS_PER_MEASURE_AT_ONE_BPM / bpm

/** The NPS range of an intensity for a player's density (plan 3.5), with the tempo, section and length limits of every chart. */
export const trainingBoundsOf = (density: TrainingDensity, intensity: TrainingIntensity) => {
    const limit = density.limit_nps_ex ?? density.max_seen_nps ?? TRAINING_DEFAULT_LIMIT_NPS
    const comfort = density.comfort_nps ?? limit * TRAINING_COMFORT_FALLBACK_RATIO
    const levels = { comfort, limit }
    const range = TRAINING_INTENSITY_RANGE[intensity]
    const withinChart = (value: number) => roundNps(clamp(value, TRAINING_NPS_FLOOR, TRAINING_NPS_CEIL))
    const npsMin = withinChart(levels[range.min.of] * range.min.ratio)
    return {
        nps_min: npsMin,
        nps_max: Math.max(npsMin, withinChart(levels[range.max.of] * range.max.ratio)),
        comfort_nps: withinChart(comfort),
        bpm_min: TRAINING_BPM_MIN,
        bpm_max: TRAINING_BPM_MAX,
        sections_min: TRAINING_LIMIT.SECTIONS_MIN,
        sections_max: TRAINING_LIMIT.SECTIONS_MAX,
        length_s: TRAINING_LENGTH_US / MICROS_PER_SECOND,
    }
}

/** The NPS range a section is held to: a rest is always light, a warm-up or cool-down stays at or under comfort. */
export const npsRangeOf = (kind: TrainingPatternKind, focusKey: string, bounds: TrainingBounds) => {
    if (kind === 'rest') return [TRAINING_REST_NPS_MIN, TRAINING_REST_NPS_MAX] as const
    if ((TRAINING_EDGE_KEYS as readonly string[]).includes(focusKey)) return [bounds.nps_min, Math.max(bounds.nps_min, bounds.comfort_nps)] as const
    return [bounds.nps_min, bounds.nps_max] as const
}

/** Whether `value` is one of the kinds of `list`. */
export const isOneOf = <T extends string>(list: readonly T[], value: string) => (list as readonly string[]).includes(value)

const NOTES_PER_ROW_OF_KIND: Partial<Record<TrainingPatternKind, number>> = {
    scratch_solo: 1 + TRAINING_SCRATCH_SOLO_KEY_SHARE,
    scratch_mixed: 1 + TRAINING_SCRATCH_MIXED_SHARE,
    scratch_run: 1 + 1 / TRAINING_RUN_ROWS.scratch_run,
    ln_release: TRAINING_LN_RELEASE_ROW_NOTES,
}

type RowShape = { kind: TrainingPatternKind; chord_size: number; scratch_ratio: number }

/** The playable notes one row of a section lays on average (plan 3.6), so `target_nps / notesPerRow` rows a second meet the target. */
export const trainingNotesPerRowOf = (section: RowShape) => {
    if (isOneOf(TRAINING_CHORD_KINDS, section.kind)) return section.chord_size
    return NOTES_PER_ROW_OF_KIND[section.kind] ?? 1 + section.scratch_ratio
}

/** Whether a kind lays runs of rows on consecutive cells (plan 3.6), which the classifier needs to see a trill, a stair or a jack. */
export const isContinuousKind = (kind: TrainingPatternKind) => Object.hasOwn(TRAINING_RUN_ROWS, kind)

/** The grids a section of a kind may be laid on, coarsest first. */
export const trainingGridsOf = (kind: TrainingPatternKind): readonly number[] => {
    if (kind === 'rest') return [TRAINING_REST_GRID]
    return isContinuousKind(kind) ? TRAINING_RUN_GRIDS : TRAINING_GRIDS
}

/** How much faster than its average a section's busiest beat is: a burst's dense beat must fit its grid. */
export const trainingPeakFactorOf = (kind: TrainingPatternKind) => (kind === 'burst' ? TRAINING_BURST_HIGH : 1)

/** Cells a second on a grid of `grid` cells a measure at `bpm`. */
export const cellsPerSecondOf = (grid: number, bpm: number) => (grid * bpm) / (BEATS_PER_MEASURE * SECONDS_PER_MINUTE)

/** The most NPS a section can lay: a row on every cell of its finest grid in its busiest beat. */
export const trainingMaxNpsOf = (section: RowShape & { bpm: number }) => {
    const grids = trainingGridsOf(section.kind)
    return (cellsPerSecondOf(grids[grids.length - 1], section.bpm) * trainingNotesPerRowOf(section)) / trainingPeakFactorOf(section.kind)
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
 * still fits while any does. The result keeps `A − min M(bpm_i) < Σ m_i × M(bpm_i) ≤ A`, every section one measure or more.
 */
export const distributeMeasures = (sections: readonly { weight: number; bpm: number }[], baseBpm: number) => {
    const available = TRAINING_LENGTH_US - measureUsOf(baseBpm) - TRAINING_TAIL_US
    const totalWeight = sections.reduce((sum, section) => sum + section.weight, 0)
    const lengths = sections.map((section) => measureUsOf(section.bpm))
    const rounded = sections.map((section, index) =>
        Math.max(TRAINING_SECTION_MIN_MEASURES, Math.round((available * section.weight) / totalWeight / lengths[index])),
    )
    const measures = [...rounded]
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
        const fitting = byWeight.find((index) => totalOf(grownAt(index)) <= available)
        added = fitting !== undefined
        if (fitting !== undefined) measures[fitting] += 1
    }
    return { rounded, measures }
}

/**
 * Turns a provider's plan into the plan the synthesiser lays out (plan 3.5). Sections whose `focus_key` is not allowed are dropped, a
 * kind that does not train its key becomes the key's first kind, and every number is held to its range; each change is a
 * correction. It runs again on every request, cached plan or not, so the corrections are the same either way. Fewer than
 * `TRAINING_LIMIT.SECTIONS_MIN` sections left is refused (`refineTrainingPlanOutput` has refused such a plan already).
 */
export const correctTrainingPlan = (plan: AiTrainingPlanOutput, context: { allowed: TrainingAllowed; bounds: TrainingBounds }) => {
    const corrections: TrainingCorrection[] = []
    const record = (section: number | null, field: TrainingCorrectionField, from: string | number, to: string | number) => {
        if (from !== to) corrections.push({ section, field, from, to })
    }
    const kept = plan.sections.flatMap((section, original) => {
        if (context.allowed.keys.includes(section.focus_key)) return [{ section, original }]
        corrections.push({ section: original, field: 'focus_key', from: section.focus_key, to: null })
        return []
    })
    if (kept.length < TRAINING_LIMIT.SECTIONS_MIN) throw createAppError('AI_OUTPUT_INVALID', { sections: kept.length })

    const baseBpm = correctBaseBpm(plan.base_bpm)
    record(null, 'base_bpm', plan.base_bpm, baseBpm)

    const laid = kept.map(({ section, original }) => {
        const kinds = trainingKindsOfKey(section.focus_key)
        const kind = kinds.length === 0 || kinds.includes(section.kind) ? section.kind : kinds[0]
        record(original, 'kind', section.kind, kind)
        const ratio = correctBpmRatio(section.bpm_ratio, baseBpm)
        record(original, 'bpm_ratio', section.bpm_ratio, ratio)
        const weight = clamp(section.weight, TRAINING_WEIGHT_MIN, TRAINING_WEIGHT_MAX)
        record(original, 'weight', section.weight, weight)
        const isChordKind = isOneOf(TRAINING_CHORD_KINDS, kind)
        const chordSize = isChordKind
            ? clamp(Math.round(section.chord_size), TRAINING_CHORD_KIND_MIN_SIZE, TRAINING_CHORD_SIZE_MAX)
            : TRAINING_CHORD_SIZE_MIN
        record(original, 'chord_size', section.chord_size, chordSize)
        const scratchRatio = isOneOf(TRAINING_SCRATCH_KINDS, kind) ? 0 : clamp(section.scratch_ratio, 0, TRAINING_MIX_RATIO_MAX)
        record(original, 'scratch_ratio', section.scratch_ratio, scratchRatio)
        const lnRatio = isOneOf(TRAINING_LN_KINDS, kind) ? 0 : clamp(section.ln_ratio, 0, TRAINING_MIX_RATIO_MAX)
        record(original, 'ln_ratio', section.ln_ratio, lnRatio)
        const bpm = baseBpm * ratio
        const [npsMin, npsMax] = npsRangeOf(kind, section.focus_key, context.bounds)
        const targetNps = Math.min(
            clamp(section.target_nps, npsMin, npsMax),
            trainingMaxNpsOf({ kind, chord_size: chordSize, scratch_ratio: scratchRatio, bpm }),
        )
        record(original, 'target_nps', section.target_nps, targetNps)
        return {
            original,
            weight,
            plan: {
                focus_key: section.focus_key,
                kind,
                lane_hint: section.lane_hint,
                chord_size: chordSize,
                scratch_ratio: scratchRatio,
                ln_ratio: lnRatio,
                bpm,
                target_nps: targetNps,
                intent: section.intent,
            },
        }
    })

    const { rounded, measures } = distributeMeasures(
        laid.map((section) => ({ weight: section.weight, bpm: section.plan.bpm })),
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
    return { spec: { base_bpm: baseBpm, sections } satisfies CorrectedTrainingPlan, corrections }
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
        sections: params.plan.sections,
        corrections: params.corrections,
    }) satisfies TrainingSpecInput
