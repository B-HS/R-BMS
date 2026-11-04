/** The most terms of the continued fraction of the incomplete beta function before it gives up converging. */
export const BETA_CF_MAX_ITERATIONS = 200
/** The continued fraction stops once a term changes it by less than this share. */
export const BETA_CF_EPSILON = 1e-12
/** A floor that keeps the continued fraction's denominators away from zero (Lentz's method). */
const BETA_CF_TINY = 1e-300

/** Lanczos approximation of ln Γ (g = 7, nine terms), accurate to about 15 digits for positive arguments. */
const LANCZOS_G = 7
const LANCZOS_COEFFICIENTS: readonly number[] = [
    0.99999999999980993, 676.5203681218851, -1259.1392167224028, 771.32342877765313, -176.61502916214059, 12.507343278686905, -0.13857109526572012,
    9.9843695780195716e-6, 1.5056327351493116e-7,
]
const HALF = 0.5
const LN_SQRT_TWO_PI = 0.5 * Math.log(2 * Math.PI)

/** ln Γ(x) for x > 0. */
export const logGamma = (x: number): number => {
    if (x < HALF) return Math.log(Math.PI / Math.sin(Math.PI * x)) - logGamma(1 - x)
    const shifted = x - 1
    const series = LANCZOS_COEFFICIENTS.slice(1).reduce(
        (sum, coefficient, index) => sum + coefficient / (shifted + index + 1),
        LANCZOS_COEFFICIENTS[0] ?? 0,
    )
    const base = shifted + LANCZOS_G + HALF
    return LN_SQRT_TWO_PI + (shifted + HALF) * Math.log(base) - base + Math.log(series)
}

/** The continued fraction of the incomplete beta function by the modified Lentz method (Numerical Recipes `betacf`). */
const betaContinuedFraction = (x: number, a: number, b: number) => {
    const sum = a + b
    const up = a + 1
    const down = a - 1
    let c = 1
    let d = 1 - (sum * x) / up
    if (Math.abs(d) < BETA_CF_TINY) d = BETA_CF_TINY
    d = 1 / d
    let fraction = d
    for (let m = 1; m <= BETA_CF_MAX_ITERATIONS; m += 1) {
        const twice = 2 * m
        const even = (m * (b - m) * x) / ((down + twice) * (a + twice))
        d = 1 + even * d
        if (Math.abs(d) < BETA_CF_TINY) d = BETA_CF_TINY
        c = 1 + even / c
        if (Math.abs(c) < BETA_CF_TINY) c = BETA_CF_TINY
        d = 1 / d
        fraction *= d * c
        const odd = (-(a + m) * (sum + m) * x) / ((a + twice) * (up + twice))
        d = 1 + odd * d
        if (Math.abs(d) < BETA_CF_TINY) d = BETA_CF_TINY
        c = 1 + odd / c
        if (Math.abs(c) < BETA_CF_TINY) c = BETA_CF_TINY
        d = 1 / d
        const change = d * c
        fraction *= change
        if (Math.abs(change - 1) < BETA_CF_EPSILON) break
    }
    return fraction
}

/** The regularized incomplete beta function I_x(a, b) for a, b > 0 and x in [0, 1]. */
export const regularizedIncompleteBeta = (x: number, a: number, b: number) => {
    if (x <= 0) return 0
    if (x >= 1) return 1
    const front = Math.exp(logGamma(a + b) - logGamma(a) - logGamma(b) + a * Math.log(x) + b * Math.log(1 - x))
    if (x < (a + 1) / (a + b + 2)) return (front * betaContinuedFraction(x, a, b)) / a
    return 1 - (front * betaContinuedFraction(1 - x, b, a)) / b
}

/** The two-sided p-value of Student's t with `df` degrees of freedom: 2 (1 − T_df(|t|)) = I_{df/(df+t²)}(df/2, 1/2). */
export const twoSidedTPValue = (t: number, df: number) => {
    if (!Number.isFinite(t)) return 0
    return Math.min(1, regularizedIncompleteBeta(df / (df + t * t), df * HALF, HALF))
}

/** The cumulative distribution of Student's t with `df` degrees of freedom. */
export const studentTCdf = (t: number, df: number) => {
    const tail = twoSidedTPValue(t, df) * HALF
    return t >= 0 ? 1 - tail : tail
}

/** Cluster values this close to their mean (relative to it, at least 1) are the same value: floating-point sums never cancel exactly. */
export const CLUSTER_FLAT_TOLERANCE = 1e-12

/** One cluster (one chart's plays) of a cell: its value and the weight it carries. */
export type ClusterValue = { value: number; weight: number }

/**
 * The weighted mean of cluster values with its cluster-robust standard error (plan 2026-11-04 2.5.3):
 * `SE = sqrt(K/(K−1)) · sqrt(Σ w² (r − R)²) / Σ w`, `t = R / SE` with K − 1 degrees of freedom and the two-sided p-value. With
 * fewer than two clusters, or a zero standard error (every cluster the same value, within CLUSTER_FLAT_TOLERANCE), there is no
 * evidence and the p-value is 1.
 */
export const clusterRobustTest = (clusters: readonly ClusterValue[]) => {
    const totalWeight = clusters.reduce((sum, cluster) => sum + cluster.weight, 0)
    const count = clusters.length
    if (count === 0 || totalWeight <= 0) return { estimate: null, se: null, t: null, df: 0, pValue: 1 }
    const estimate = clusters.reduce((sum, cluster) => sum + cluster.weight * cluster.value, 0) / totalWeight
    if (count < 2) return { estimate, se: null, t: null, df: 0, pValue: 1 }
    const tolerance = CLUSTER_FLAT_TOLERANCE * Math.max(1, Math.abs(estimate))
    if (clusters.every((cluster) => Math.abs(cluster.value - estimate) <= tolerance)) return { estimate, se: 0, t: null, df: count - 1, pValue: 1 }
    const spread = clusters.reduce((sum, cluster) => sum + cluster.weight ** 2 * (cluster.value - estimate) ** 2, 0)
    const se = (Math.sqrt(count / (count - 1)) * Math.sqrt(spread)) / totalWeight
    const df = count - 1
    if (se <= 0) return { estimate, se, t: null, df, pValue: 1 }
    const t = estimate / se
    return { estimate, se, t, df, pValue: twoSidedTPValue(t, df) }
}

/**
 * Benjamini–Hochberg at false discovery rate `q`: the p-values sorted up, every one up to the largest rank i with p_(i) ≤ i/m · q
 * passes. `threshold` is each p-value's own critical value, its rank over m times q (ties ranked in input order).
 */
export const benjaminiHochberg = (pValues: readonly number[], q: number) => {
    const count = pValues.length
    const order = pValues
        .map((pValue, index) => ({ pValue, index }))
        .toSorted((left, right) => left.pValue - right.pValue || left.index - right.index)
    const rankOf = new Map(order.map((entry, rank) => [entry.index, rank + 1]))
    const cutoff = order.reduce((largest, entry, rank) => (entry.pValue <= ((rank + 1) / count) * q ? rank + 1 : largest), 0)
    return pValues.map((_, index) => {
        const rank = rankOf.get(index) ?? count
        return { passed: rank <= cutoff, threshold: (rank / count) * q }
    })
}

export const MONOTONE = { INCREASING: 'increasing', DECREASING: 'decreasing' } as const
export type Monotone = (typeof MONOTONE)[keyof typeof MONOTONE]

/**
 * The weighted isotonic regression of `values` in their order by the pool adjacent violators algorithm: the closest (weighted least
 * squares) sequence that never goes down (`increasing`) or never goes up (`decreasing`).
 */
export const isotonicRegression = (values: readonly number[], weights: readonly number[], direction: Monotone) => {
    const sign = direction === MONOTONE.INCREASING ? 1 : -1
    const blocks: { mean: number; weight: number; size: number }[] = []
    values.forEach((value, index) => {
        let block = { mean: sign * value, weight: weights[index] ?? 0, size: 1 }
        while (blocks.length > 0 && (blocks.at(-1)?.mean ?? -Infinity) > block.mean) {
            const previous = blocks.pop()
            if (!previous) break
            const weight = previous.weight + block.weight
            const mean = weight > 0 ? (previous.mean * previous.weight + block.mean * block.weight) / weight : (previous.mean + block.mean) * HALF
            block = { mean, weight, size: previous.size + block.size }
        }
        blocks.push(block)
    })
    return blocks.flatMap((block) => Array.from({ length: block.size }, () => sign * block.mean))
}

/** One point of a fitted curve: its position along the NPS axis and the fitted value there. */
export type CurvePoint = { x: number; y: number }

/**
 * The first place a fitted curve reaches `level`: a `decreasing` curve once it is at or below it, an `increasing` one once it is at
 * or above it. `index` is the first point that has reached it; `x` is where the straight line from the point before it crosses the
 * level, or that first point's own `x` when no point comes before it (the curve is past the level from its start). `null` when the
 * curve never reaches the level.
 */
export const firstCrossingOf = (points: readonly CurvePoint[], level: number, direction: Monotone) => {
    const hasReached = (y: number) => (direction === MONOTONE.DECREASING ? y <= level : y >= level)
    const index = points.findIndex((point) => hasReached(point.y))
    if (index < 0) return null
    const reached = points[index]
    const before = points[index - 1]
    if (!before) return { index, x: reached.x }
    return { index, x: before.x + ((before.y - level) / (before.y - reached.y)) * (reached.x - before.x) }
}
