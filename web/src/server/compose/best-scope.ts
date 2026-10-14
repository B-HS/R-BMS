import { and, eq, ne, notExists, or, sql } from 'drizzle-orm'
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
