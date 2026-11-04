import type { TrainingShape } from '@server/dto/training'
import { mulberry32, type RandomSource } from '@server/service/domain/training/prng'
import {
    TRAINING_CLIPPED_SHARE_MAX,
    TRAINING_CURVE_CV,
    TRAINING_CURVE_NOISE_CORRELATION,
    TRAINING_CURVE_NOISE_SD,
    TRAINING_CURVE_NORMAL_DRAWS,
    TRAINING_CURVE_PART_SAMPLES,
    TRAINING_CURVE_PEAK_STEPS,
    TRAINING_CURVE_PEAK_RATIO,
    TRAINING_CURVE_POINT_FLOOR,
    TRAINING_CURVE_SEED_SALT,
    TRAINING_CURVE_STEP_US,
    TRAINING_DENSITY_WINDOW_US,
    TRAINING_BODY_PERCENTILE,
    TRAINING_SHAPE_POINTS,
} from '@server/service/domain/training/training-constants'

const HALF = 0.5
const MEAN_SOLVE_ROUNDS = 60
const MEAN_SEARCH_HEADROOM = 4

const meanOf = (values: readonly number[]) => values.reduce((sum, value) => sum + value, 0) / values.length

/** The value at the nearest-rank percentile `share` of `values` with `weights` (the analysis engine's percentile). */
export const weightedPercentileOf = (values: readonly number[], weights: readonly number[], share: number) => {
    const order = values.map((_, index) => index).toSorted((a, b) => values[a] - values[b])
    const total = weights.reduce((sum, weight) => sum + weight, 0)
    let gathered = 0
    for (const index of order) {
        gathered += weights[index]
        if (gathered >= share * total) return values[index]
    }
    return values[order[order.length - 1]] ?? 0
}

/** The nearest-rank percentile `share` of `values` counted one each, as the engine reads a density percentile of notes. */
export const nearestRankOf = (values: readonly number[], share: number) => {
    const sorted = values.toSorted((a, b) => a - b)
    return sorted[Math.max(0, Math.ceil(share * sorted.length) - 1)] ?? 0
}

/** The seed of a chart's curve noise: the chart seed mixed apart from every section's generator. */
export const trainingCurveSeedOf = (seed: number) => (seed ^ TRAINING_CURVE_SEED_SALT) >>> 0

/** A standard normal draw as the sum of TRAINING_CURVE_NORMAL_DRAWS uniform draws, centred: arithmetic only, the same on every runtime. */
const standardNormalOf = (random: RandomSource) =>
    Array.from({ length: TRAINING_CURVE_NORMAL_DRAWS }, () => random()).reduce((sum, value) => sum + value, 0) - TRAINING_CURVE_NORMAL_DRAWS * HALF

/** The curve at `position` parts from the body's start: the points at the centres of the parts, linear between, flat past the ends. */
const curveValueAt = (points: readonly number[], position: number) => {
    const at = position - HALF
    if (at <= 0) return points[0]
    if (at >= points.length - 1) return points[points.length - 1]
    const low = Math.floor(at)
    return points[low] + (points[low + 1] - points[low]) * (at - low)
}

const variationOf = (values: readonly number[]) => {
    const mean = meanOf(values)
    return Math.sqrt(meanOf(values.map((value) => (value - mean) ** 2))) / mean
}

/** The mean of the curve over each of its parts, as twelve equal parts of a chart's notes count them. */
const partMeansOf = (points: readonly number[]) =>
    points.map((_, part) =>
        meanOf(
            Array.from({ length: TRAINING_CURVE_PART_SAMPLES }, (_, sample) =>
                curveValueAt(points, part + (sample + HALF) / TRAINING_CURVE_PART_SAMPLES),
            ),
        ),
    )

/**
 * The twelve points of a body's density curve, mean 1 (M4): the shape's points (05 9.4) moved by noise of TRAINING_CURVE_NOISE_SD
 * whose neighbours correlate TRAINING_CURVE_NOISE_CORRELATION, floored and rescaled to mean 1, their spread scaled so the curve's
 * twelve part means vary by TRAINING_CURVE_CV.
 */
export const trainingCurvePointsOf = (shape: TrainingShape, noiseSeed: number) => {
    const random = mulberry32(noiseSeed)
    const innovation = TRAINING_CURVE_NOISE_SD * Math.sqrt(1 - TRAINING_CURVE_NOISE_CORRELATION ** 2)
    const noise = TRAINING_SHAPE_POINTS[shape].reduce<number[]>((drawn) => {
        const draw = standardNormalOf(random)
        const previous = drawn.at(-1)
        return [...drawn, previous === undefined ? TRAINING_CURVE_NOISE_SD * draw : TRAINING_CURVE_NOISE_CORRELATION * previous + innovation * draw]
    }, [])
    const raw = TRAINING_SHAPE_POINTS[shape].map((point, index) => Math.max(TRAINING_CURVE_POINT_FLOOR, point + noise[index]))
    const rawMean = meanOf(raw)
    const normal = raw.map((point) => point / rawMean)
    const spread = variationOf(partMeansOf(normal))
    const scale = spread > 0 ? Math.min(TRAINING_CURVE_CV.max, Math.max(TRAINING_CURVE_CV.min, spread)) / spread : 1
    return normal.map((point) => Math.max(TRAINING_CURVE_POINT_FLOOR, 1 + (point - 1) * scale))
}

/**
 * The step values with a short peak (05 6.4: a real minute has about three moments past 1.25μ, two seconds long at the median): when
 * the highest step stands less than TRAINING_CURVE_PEAK_RATIO over the note-weighted median, the TRAINING_CURVE_PEAK_STEPS steps from
 * it are lifted to that ratio and the step on each side of them halfway, a peak short enough to leave the body's p90 where it was.
 */
const withPeakOf = (values: readonly number[]) => {
    const peak = weightedPercentileOf(values, values, HALF) * TRAINING_CURVE_PEAK_RATIO
    const highest = Math.max(...values)
    if (highest >= peak) return [...values]
    const first = Math.max(0, Math.min(values.indexOf(highest), values.length - TRAINING_CURVE_PEAK_STEPS))
    const last = first + TRAINING_CURVE_PEAK_STEPS - 1
    return values.map((value, step) => {
        if (step >= first && step <= last) return peak
        return step === first - 1 || step === last + 1 ? Math.max(value, (value + peak) * HALF) : value
    })
}

/**
 * The curve's factor at any time of the body `[fromUs, toUs)`: the points sit at the centres of twelve equal parts of the body and are
 * read by linear interpolation at the centre of each TRAINING_CURVE_STEP_US step, flat before the first centre and after the last, with
 * a short peak on the highest step.
 */
export const trainingCurveOf = (points: readonly number[], fromUs: number, toUs: number) => {
    const span = Math.max(1, toUs - fromUs)
    const partUs = span / points.length
    const steps = Math.max(1, Math.ceil(span / TRAINING_CURVE_STEP_US))
    const values = withPeakOf(Array.from({ length: steps }, (_, step) => curveValueAt(points, ((step + HALF) * TRAINING_CURVE_STEP_US) / partUs)))
    const stepAt = (timeUs: number) => Math.min(steps - 1, Math.max(0, Math.floor((timeUs - fromUs) / TRAINING_CURVE_STEP_US)))
    return { fromUs, toUs, steps, values, factorAt: (timeUs: number) => values[stepAt(timeUs)] }
}

export type TrainingCurve = ReturnType<typeof trainingCurveOf>

const WINDOW_STEPS = TRAINING_DENSITY_WINDOW_US / TRAINING_CURVE_STEP_US

/** Each step's density as a centred 2-second window reads it: the steps the window covers, the two at its ends by half. */
const windowedOf = (densities: readonly number[]) =>
    densities.map((_, step) => {
        const reach = WINDOW_STEPS / 2
        const taps = Array.from({ length: WINDOW_STEPS + 1 }, (_, tap) => ({
            at: step - reach + tap,
            weight: tap === 0 || tap === WINDOW_STEPS ? HALF : 1,
        }))
        const inside = taps.filter((tap) => tap.at >= 0 && tap.at < densities.length)
        const weight = inside.reduce((sum, tap) => sum + tap.weight, 0)
        return inside.reduce((sum, tap) => sum + densities[tap.at] * tap.weight, 0) / weight
    })

/**
 * The body mean μ that brings the body's note-weighted nps2 p90 to `p90Target` (M1, M4): the density at a time is `μ × factor`, cut to
 * the ceiling of the section laid there and moved by its modulation (a burst's measures), read through a 2-second window and weighted
 * by the notes it lays. μ is found by bisection; it is then kept to the most that leaves no more than TRAINING_CLIPPED_SHARE_MAX of the
 * body cut, so a ceiling below the target keeps the curve's shape under it rather than flattening it. Returns μ, that most, and the
 * p90 μ gives.
 */
export const solveBodyMean = (
    curve: TrainingCurve,
    layout: { ceilingAt: (timeUs: number) => number; modulationAt: (timeUs: number) => number },
    p90Target: number,
) => {
    const steps = Array.from({ length: curve.steps }, (_, step) => {
        const fromUs = curve.fromUs + step * TRAINING_CURVE_STEP_US
        const timeUs = fromUs + HALF * TRAINING_CURVE_STEP_US
        return {
            factor: curve.values[step],
            ceiling: layout.ceilingAt(timeUs),
            modulation: layout.modulationAt(timeUs),
            lengthUs: Math.max(0, Math.min(TRAINING_CURVE_STEP_US, curve.toUs - fromUs)),
        }
    })
    const p90Of = (mean: number) => {
        const densities = steps.map((step) => Math.min(mean * step.factor, step.ceiling) * step.modulation)
        return weightedPercentileOf(
            windowedOf(densities),
            densities.map((density, index) => density * steps[index].lengthUs),
            TRAINING_BODY_PERCENTILE,
        )
    }
    const lowestFactor = Math.min(...steps.map((step) => step.factor))
    let low = 0
    let high = (p90Target / lowestFactor) * MEAN_SEARCH_HEADROOM
    for (let round = 0; round < MEAN_SOLVE_ROUNDS; round += 1) {
        const middle = (low + high) * HALF
        if (p90Of(middle) < p90Target) low = middle
        else high = middle
    }
    const byHeadroom = steps.map((step) => ({ ratio: step.ceiling / step.factor, lengthUs: step.lengthUs })).toSorted((a, b) => a.ratio - b.ratio)
    const bodyUs = steps.reduce((sum, step) => sum + step.lengthUs, 0)
    let cutUs = 0
    const keepShape = byHeadroom.find((step) => {
        cutUs += step.lengthUs
        return cutUs > TRAINING_CLIPPED_SHARE_MAX * bodyUs
    })
    const meanMax = keepShape?.ratio ?? Number.POSITIVE_INFINITY
    const mean = Math.min(high, meanMax)
    return { mean, meanMax, p90: p90Of(mean) }
}
