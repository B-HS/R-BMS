import { and, asc, desc, eq, gt, lt, ne, notExists, or, sql } from 'drizzle-orm'
import { alias, type MySqlColumn, type MySqlTable } from 'drizzle-orm/mysql-core'
import type { Database } from '@server/db'
import { chartBest, courseBest } from '@server/db/schema'

type BestColumns = {
    target: MySqlColumn
    userId: MySqlColumn
    clear: MySqlColumn
    exScore: MySqlColumn
    minbp: MySqlColumn
    scratchAuto: MySqlColumn
}

const outranks = (candidate: BestColumns, other: BestColumns) =>
    or(
        sql`${candidate.clear} > ${other.clear}`,
        and(eq(candidate.clear, other.clear), sql`${candidate.exScore} > ${other.exScore}`),
        and(eq(candidate.clear, other.clear), eq(candidate.exScore, other.exScore), sql`${candidate.minbp} < ${other.minbp}`),
        and(
            eq(candidate.clear, other.clear),
            eq(candidate.exScore, other.exScore),
            eq(candidate.minbp, other.minbp),
            eq(candidate.scratchAuto, false),
        ),
    )

const bestScopeFilter = (params: { db: Database; row: BestColumns; siblingTable: MySqlTable; sibling: BestColumns; includeScratchAuto: boolean }) => {
    if (!params.includeScratchAuto) return eq(params.row.scratchAuto, false)
    const { row, sibling } = params
    return notExists(
        params.db
            .select({ one: sql`1` })
            .from(params.siblingTable)
            .where(
                and(eq(sibling.target, row.target), eq(sibling.userId, row.userId), ne(sibling.scratchAuto, row.scratchAuto), outranks(sibling, row)),
            ),
    )
}

const chartBestSibling = alias(chartBest, 'chart_best_sibling')
const courseBestSibling = alias(courseBest, 'course_best_sibling')

const chartBestColumns = (table: typeof chartBest | typeof chartBestSibling): BestColumns => ({
    target: table.chartSha256,
    userId: table.userId,
    clear: table.clear,
    exScore: table.exScore,
    minbp: table.minbp,
    scratchAuto: table.scratchAuto,
})

const courseBestColumns = (table: typeof courseBest | typeof courseBestSibling): BestColumns => ({
    target: table.courseHash,
    userId: table.userId,
    clear: table.clear,
    exScore: table.exScore,
    minbp: table.minbp,
    scratchAuto: table.scratchAuto,
})

/**
 * Keeps the chart_best rows a list shows: the normal-play bests only, or, with SCRATCH AUTO included,
 * the better of each player's normal and SCRATCH AUTO best (the normal one on a tie).
 */
export const chartBestScope = (db: Database, includeScratchAuto: boolean) =>
    bestScopeFilter({
        db,
        row: chartBestColumns(chartBest),
        siblingTable: chartBestSibling,
        sibling: chartBestColumns(chartBestSibling),
        includeScratchAuto,
    })

/**
 * The course_best counterpart of {@link chartBestScope}.
 */
export const courseBestScope = (db: Database, includeScratchAuto: boolean) =>
    bestScopeFilter({
        db,
        row: courseBestColumns(courseBest),
        siblingTable: courseBestSibling,
        sibling: courseBestColumns(courseBestSibling),
        includeScratchAuto,
    })

type BestScore = { clear: number; exScore: number; minbp: number }

/**
 * The stored best rows that a candidate run beats: a higher lamp, or the same lamp with a higher EX, or both the same with fewer BP.
 * An UPDATE that carries this as its condition replaces a best only when the run really is better, whatever order two runs land in.
 */
export const isBeatenBy = (stored: Pick<BestColumns, 'clear' | 'exScore' | 'minbp'>, candidate: BestScore) =>
    or(
        lt(stored.clear, candidate.clear),
        and(eq(stored.clear, candidate.clear), lt(stored.exScore, candidate.exScore)),
        and(eq(stored.clear, candidate.clear), eq(stored.exScore, candidate.exScore), gt(stored.minbp, candidate.minbp)),
    )

/**
 * The best rows that rank strictly ahead of a score: ties on lamp, EX and BP share one rank, so they do not count.
 */
export const isRankedAheadOf = (row: Pick<BestColumns, 'clear' | 'exScore' | 'minbp'>, score: BestScore) =>
    or(
        gt(row.clear, score.clear),
        and(eq(row.clear, score.clear), gt(row.exScore, score.exScore)),
        and(eq(row.clear, score.clear), eq(row.exScore, score.exScore), lt(row.minbp, score.minbp)),
    )

/**
 * `RANK()` over the rows a list keeps: tied lamp, EX and BP share a rank and the next one skips, the same number a submit
 * answer counts with {@link isRankedAheadOf}.
 */
export const rankInList = (row: Pick<BestColumns, 'clear' | 'exScore' | 'minbp'>) =>
    sql<number>`rank() over (order by ${row.clear} desc, ${row.exScore} desc, ${row.minbp} asc)`.mapWith(Number)

/**
 * The full list order. Rows tied on lamp, EX and BP keep the order the earlier best came first, then by player, so a page
 * boundary never repeats or skips a player.
 */
export const listOrder = (row: Pick<BestColumns, 'clear' | 'exScore' | 'minbp' | 'userId'> & { updatedAt: MySqlColumn }) => [
    desc(row.clear),
    desc(row.exScore),
    asc(row.minbp),
    asc(row.updatedAt),
    asc(row.userId),
]
