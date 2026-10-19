import { CHART_TAGS_NOT_SHOWN, MILLIS_DECIMALS } from '@server/service/domain/ai/score-feature.constants'
import { roundRatio } from '@server/service/domain/analysis/chart-features'
import type { Tally } from '@server/service/domain/analysis/play-metrics'
import type { AnalyzedPlay } from '@server/service/domain/trace/trace.service'

const EX_PER_NOTE = 2
const MICROS_PER_MILLI = 1000
const HIDDEN_TAGS = new Set<string>(CHART_TAGS_NOT_SHOWN)

/** A pooled tally: notes that took a judgement, their EX score and the FAST/SLOW of GREAT to BAD (PGREAT has no side worth telling). */
export type PooledTally = { notes: number; ex_score: number; fast: number; slow: number }

const EMPTY_POOL: PooledTally = { notes: 0, ex_score: 0, fast: 0, slow: 0 }

const sideOf = (side: Tally['fast']) => side.great + side.good + side.bad

/** Adds one play's tally to a pool. */
export const addToPool = (pool: PooledTally | undefined, tally: Tally) => ({
    notes: (pool ?? EMPTY_POOL).notes + tally.notes,
    ex_score: (pool ?? EMPTY_POOL).ex_score + tally.ex_score,
    fast: (pool ?? EMPTY_POOL).fast + sideOf(tally.fast),
    slow: (pool ?? EMPTY_POOL).slow + sideOf(tally.slow),
})

/** Pools the tallies of one group (`by_lane`, `by_density`, ...) over plays, by group name. */
export const poolGroups = (groups: readonly Readonly<Record<string, Tally>>[]) => {
    const pools = new Map<string, PooledTally>()
    for (const group of groups) for (const [name, tally] of Object.entries(group)) pools.set(name, addToPool(pools.get(name), tally))
    return pools
}

/** EX ratio `(2 PG + GR) / (2 N)` of a pool, null with no note. */
export const exRatioOf = (pool: Pick<PooledTally, 'notes' | 'ex_score'>) =>
    pool.notes > 0 ? roundRatio(pool.ex_score / (EX_PER_NOTE * pool.notes)) : null

/** The FAST share of a pool's GREAT to BAD judgements, null with none. */
export const fastShareOf = (pool: Pick<PooledTally, 'fast' | 'slow'>) =>
    pool.fast + pool.slow > 0 ? roundRatio(pool.fast / (pool.fast + pool.slow)) : null

export const roundTo = (value: number, decimals: number) => {
    const scale = 10 ** decimals
    return Math.round(value * scale) / scale
}

/** Microseconds as milliseconds with MILLIS_DECIMALS decimals. */
export const usToMs = (us: number) => roundTo(us / MICROS_PER_MILLI, MILLIS_DECIMALS)

/** A play's tags worth naming: CHART_TAGS_NOT_SHOWN say little on their own. */
export const isShownTag = (tag: string) => !HIDDEN_TAGS.has(tag)

/** The analysed plays the per-note figures exist for (7-key single play), each with its metrics. */
export const playsWithMetrics = (plays: readonly AnalyzedPlay[]) =>
    plays.flatMap((play) => (play.summary.metrics ? [{ play, metrics: play.summary.metrics }] : []))

/** The digest key of a segment. */
export const segmentKey = (id: string) => `seg.${id}`
