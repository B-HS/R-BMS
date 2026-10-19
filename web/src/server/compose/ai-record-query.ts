import { and, asc, desc, eq, gte, inArray, like, lte, ne, notInArray, sql, type SQL } from 'drizzle-orm'
import type { MySqlColumn } from 'drizzle-orm/mysql-core'
import type { Database } from '@server/db'
import { chart, chartBest, score } from '@server/db/schema'
import { AI_RECORD_QUERY_LIMIT } from '@server/dto/ai'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import {
    EX_RATE_BASIS_POINTS_PER_PERCENT,
    type RecordCondition,
    type RecordQuerySort,
    type RecordQueryTable,
} from '@server/service/domain/ai/record-query.compiler'
import { createAiRecordQueryService, type AiRecordQueryServiceDb } from '@server/service/domain/ai/record-query.service'
import { EX_PER_PGREAT } from '@server/service/domain/score/best-policy'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { orderedLevels } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { chartBestScope } from '@server/compose/best-scope'

const FULL_EX_RATE_BASIS_POINTS = AI_RECORD_QUERY_LIMIT.EX_RATE_MAX_PERCENT * EX_RATE_BASIS_POINTS_PER_PERCENT
const LIKE_WILDCARDS = /[\\%_]/g

const escapeLike = (value: string) => value.replace(LIKE_WILDCARDS, (character) => `\\${character}`)

type RecordColumns = { clear: MySqlColumn; exScore: MySqlColumn; minbp: MySqlColumn; scratchAuto: MySqlColumn }

const exScoreDenominator = () => sql`(case when ${score.maxExScore} > 0 then ${score.maxExScore} else ${score.notes} * ${EX_PER_PGREAT} end)`

const compareNumber = (column: MySqlColumn, op: 'eq' | 'ne' | 'gte' | 'lte' | 'in', values: readonly (number | string)[]) => {
    const [first = 0] = values
    if (op === 'eq') return eq(column, first)
    if (op === 'ne') return ne(column, first)
    if (op === 'gte') return gte(column, first)
    if (op === 'lte') return lte(column, first)
    return inArray(column, [...values])
}

const toWhere = (condition: RecordCondition, columns: RecordColumns) => {
    if (condition.field === 'lamp') return compareNumber(columns.clear, condition.op, condition.values)
    if (condition.field === 'gauge') return compareNumber(score.gauge, condition.op, condition.values)
    if (condition.field === 'random') {
        if (condition.op === 'ne') return notInArray(score.random, condition.values)
        return inArray(score.random, condition.values)
    }
    if (condition.field === 'bp') return compareNumber(columns.minbp, condition.op, [condition.value])
    if (condition.field === 'played_at') return compareNumber(score.playedAt, condition.op, [condition.at])
    if (condition.field === 'scratch_auto') return eq(columns.scratchAuto, condition.value)
    if (condition.field === 'title') return like(chart.title, `%${escapeLike(condition.text)}%`)
    const scaledExScore = sql`${columns.exScore} * ${FULL_EX_RATE_BASIS_POINTS}`
    const scaledBound = sql`${condition.basisPoints} * ${exScoreDenominator()}`
    return and(
        sql`${exScoreDenominator()} > 0`,
        condition.op === 'gte' ? sql`${scaledExScore} >= ${scaledBound}` : sql`${scaledExScore} <= ${scaledBound}`,
    )
}

const SCORE_COLUMNS: RecordColumns = { clear: score.clear, exScore: score.exScore, minbp: score.minbp, scratchAuto: score.scratchAuto }
const BEST_COLUMNS: RecordColumns = { clear: chartBest.clear, exScore: chartBest.exScore, minbp: chartBest.minbp, scratchAuto: chartBest.scratchAuto }

const SCORE_SORT = {
    played_at: score.playedAt,
    ex_rate: sql`${score.exScore} / nullif(${exScoreDenominator()}, 0)`,
    bp: score.minbp,
    level: chart.level,
    lamp: score.clear,
} satisfies Record<RecordQuerySort['sort_field'], MySqlColumn | SQL>

const recordSelection = (columns: RecordColumns) => ({
    scoreId: score.id,
    chartSha256: score.chartSha256,
    chartMd5: chart.md5,
    title: chart.title,
    artist: chart.artist,
    clear: columns.clear,
    exScore: columns.exScore,
    maxExScore: exScoreDenominator().mapWith(Number),
    minbp: columns.minbp,
    playedAt: score.playedAt,
    random: score.random,
    gauge: score.gauge,
    scratchAuto: columns.scratchAuto,
})

/** The table snapshot in the shape the record query compiler reads: levels in table order. */
export const toRecordQueryTables = (tables: Map<string, SnapshotTable>): RecordQueryTable[] =>
    [...tables.values()].map((table) => ({ id: table.id, symbol: table.header.symbol, levels: orderedLevels(table), entries: table.entries }))

/**
 * The record query reads with every value bound as a parameter: the compiled conditions become Drizzle operators, never SQL text.
 * Every query is pinned to the session user's id.
 */
export const createAiRecordQueryDb = (db: Database): AiRecordQueryServiceDb => ({
    listScoreRecords: async ({ userId, conditions, sort, limit }) => {
        const order = sort.sort_dir === 'asc' ? asc : desc
        const rows = await db
            .select(recordSelection(SCORE_COLUMNS))
            .from(score)
            .leftJoin(chart, eq(chart.sha256, score.chartSha256))
            .where(and(eq(score.userId, userId), ...conditions.map((condition) => toWhere(condition, SCORE_COLUMNS))))
            .orderBy(order(SCORE_SORT[sort.sort_field]), desc(score.playedAt), desc(score.id))
            .limit(limit)
        return rows.map((row) => ({ ...row, title: row.title ?? '', artist: row.artist ?? '' }))
    },

    listTableBestRecords: async ({ userId, md5s, conditions }) => {
        if (md5s.length === 0) return []
        const scratchAutoChosen = conditions.some((condition) => condition.field === 'scratch_auto')
        const rows = await db
            .select(recordSelection(BEST_COLUMNS))
            .from(chartBest)
            .innerJoin(chart, eq(chart.sha256, chartBest.chartSha256))
            .innerJoin(score, eq(score.id, chartBest.scoreId))
            .where(
                and(
                    eq(chartBest.userId, userId),
                    scratchAutoChosen ? undefined : chartBestScope(db, true),
                    inArray(chart.md5, md5s),
                    ...conditions.map((condition) => toWhere(condition, BEST_COLUMNS)),
                ),
            )
        return rows.map((row) => ({ ...row, title: row.title ?? '', artist: row.artist ?? '' }))
    },

    listPlayedMd5s: async ({ userId, md5s }) => {
        if (md5s.length === 0) return []
        const rows = await db
            .selectDistinct({ md5: chart.md5 })
            .from(chartBest)
            .innerJoin(chart, eq(chart.sha256, chartBest.chartSha256))
            .where(and(eq(chartBest.userId, userId), inArray(chart.md5, md5s)))
        return rows.flatMap((row) => (row.md5 ? [row.md5] : []))
    },
})

export const composeAiRecordQuery = (
    db: Database,
    deps: { aiFeatureRunner: Pick<AiFeatureRunner, 'run'>; loadTables: () => Map<string, SnapshotTable> },
) => ({
    aiRecordQueryService: createAiRecordQueryService({
        runner: deps.aiFeatureRunner,
        db: createAiRecordQueryDb(db),
        loadTables: () => toRecordQueryTables(deps.loadTables()),
        now: () => Date.now(),
    }),
})
