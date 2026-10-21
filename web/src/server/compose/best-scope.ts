import { and, asc, desc, eq, ne, notExists, or, sql } from 'drizzle-orm'
import { alias, type MySqlColumn, type MySqlTable } from 'drizzle-orm/mysql-core'
import type { Database } from '@server/db'
import { chartBest, courseBest } from '@server/db/schema'

type BestColumns = {
    target: MySqlColumn
    userId: MySqlColumn
    clear: MySqlColumn
    assistGrade: MySqlColumn
    exScore: MySqlColumn
    minbp: MySqlColumn
    scratchAuto: MySqlColumn
}

type BestScore = { clear: number; assistGrade: number; exScore: number; minbp: number }

type BestKey = keyof BestScore

type BestSide = Record<BestKey, MySqlColumn | number>

/**
 * The keys a best is ranked by, in the order they are compared: a higher lamp, then a higher assist grade, then a higher EX,
 * then fewer BP. The assist grade only splits LIGHT ASSIST clears, so it never lifts a run over a higher lamp.
 */
const BEST_ORDER = [
    { key: 'clear', isHigherBetter: true },
    { key: 'assistGrade', isHigherBetter: true },
    { key: 'exScore', isHigherBetter: true },
    { key: 'minbp', isHigherBetter: false },
] as const satisfies readonly { key: BestKey; isHigherBetter: boolean }[]

const isSameOn = (left: BestSide, right: BestSide, key: BestKey) => sql`${left[key]} = ${right[key]}`

/**
 * Whether `ahead` ranks strictly above `behind`: the first key they differ on decides, in the BEST_ORDER direction.
 * Each side is a table's columns or the plain numbers of a run.
 */
const isRankedAbove = (ahead: BestSide, behind: BestSide) =>
    or(
        ...BEST_ORDER.map(({ key, isHigherBetter }, index) =>
            and(
                ...BEST_ORDER.slice(0, index).map((prior) => isSameOn(ahead, behind, prior.key)),
                isHigherBetter ? sql`${ahead[key]} > ${behind[key]}` : sql`${ahead[key]} < ${behind[key]}`,
            ),
        ),
    )

const isTiedWith = (left: BestSide, right: BestSide) => and(...BEST_ORDER.map(({ key }) => isSameOn(left, right, key)))

const outranks = (candidate: BestColumns, other: BestColumns) =>
    or(isRankedAbove(candidate, other), and(isTiedWith(candidate, other), eq(candidate.scratchAuto, false)))

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
    assistGrade: table.assistGrade,
    exScore: table.exScore,
    minbp: table.minbp,
    scratchAuto: table.scratchAuto,
})

const courseBestColumns = (table: typeof courseBest | typeof courseBestSibling): BestColumns => ({
    target: table.courseHash,
    userId: table.userId,
    clear: table.clear,
    assistGrade: table.assistGrade,
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

type BestColumnsOfRank = Pick<BestColumns, BestKey>

/**
 * The stored best rows that a candidate run beats: a higher lamp, or the same lamp with a higher assist grade, or both the same with
 * a higher EX, or all three the same with fewer BP. An UPDATE that carries this as its condition replaces a best only when the run
 * really is better, whatever order two runs land in.
 */
export const isBeatenBy = (stored: BestColumnsOfRank, candidate: BestScore) => isRankedAbove(candidate, stored)

/**
 * The best rows that rank strictly ahead of a score: ties on lamp, assist grade, EX and BP share one rank, so they do not count.
 */
export const isRankedAheadOf = (row: BestColumnsOfRank, score: BestScore) => isRankedAbove(row, score)

/**
 * `RANK()` over the rows a list keeps: tied lamp, assist grade, EX and BP share a rank and the next one skips, the same number a
 * submit answer counts with {@link isRankedAheadOf}.
 */
export const rankInList = (row: BestColumnsOfRank) =>
    sql<number>`rank() over (order by ${row.clear} desc, ${row.assistGrade} desc, ${row.exScore} desc, ${row.minbp} asc)`.mapWith(Number)

/**
 * The full list order. Rows tied on lamp, assist grade, EX and BP keep the order the earlier best came first, then by player, so a
 * page boundary never repeats or skips a player.
 */
export const listOrder = (row: BestColumnsOfRank & Pick<BestColumns, 'userId'> & { updatedAt: MySqlColumn }) => [
    desc(row.clear),
    desc(row.assistGrade),
    desc(row.exScore),
    asc(row.minbp),
    asc(row.updatedAt),
    asc(row.userId),
]
