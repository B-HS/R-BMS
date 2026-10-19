import { and, asc, count, countDistinct, desc, eq, gt, inArray, isNull, ne, or, sql, sum } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { playAnalysis, playShareSetting, playTrace, rival, score } from '@server/db/schema'
import { summarizeError } from '@server/lib/error-summary'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { newId } from '@server/lib/token'
import { withLockRetry } from '@server/lib/with-lock-retry'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { exScoreDenominator } from '@server/compose/ai-record-query'
import { ANALYSIS_FAILED_JOIN, type AnalysisVersions } from '@server/service/domain/trace/trace-analysis'
import { createTraceService, type TracedPlayRow, type TraceServiceDb } from '@server/service/domain/trace/trace.service'

const scoreSelection = {
    id: score.id,
    userId: score.userId,
    chartSha256: score.chartSha256,
    chartMd5: score.chartMd5,
    mode: score.mode,
    lntype: score.lntype,
    random: score.random,
    scratchAuto: score.scratchAuto,
    playedAt: score.playedAt,
    pgreat: score.pgreat,
    great: score.great,
    good: score.good,
    bad: score.bad,
    poor: score.poor,
    epg: score.epg,
    lpg: score.lpg,
    egr: score.egr,
    lgr: score.lgr,
    egd: score.egd,
    lgd: score.lgd,
    ebd: score.ebd,
    lbd: score.lbd,
    emptyPoor: score.emptyPoor,
}

const tracedPlaySelection = {
    traceId: playTrace.id,
    scoreId: playTrace.scoreId,
    chartSha256: playTrace.chartSha256,
    chartMd5: playTrace.chartMd5,
    playedAt: playTrace.playedAt,
    clear: score.clear,
    exScore: score.exScore,
    maxExScore: exScoreDenominator().mapWith(Number),
    minbp: score.minbp,
    gauge: score.gauge,
    random: score.random,
    scratchAuto: score.scratchAuto,
    judgeRate: score.judgeRate,
    assist: score.assist,
    autoplay: score.autoplay,
    autoOffset: score.autoOffset,
    constant: score.constant,
    greenNumber: score.greenNumber,
    analysisVersion: playAnalysis.analysisVersion,
    summary: playAnalysis.summary,
}

const storedPatternVersion = sql`coalesce(cast(json_unquote(json_extract(${playAnalysis.summary}, '$.pattern_analysis_version')) as signed), 0)`

const isBehindVersions = (versions: AnalysisVersions) =>
    sql`(${playAnalysis.analysisVersion} < ${versions.play} or ${storedPatternVersion} < ${versions.pattern})`

const traceTotals = {
    traces: count(),
    bytes: sum(playTrace.byteSize),
    storedBytes: sum(playTrace.storedSize),
}

export const createTraceDb = (db: Database) => {
    const selectTracedPlays = () =>
        db
            .select(tracedPlaySelection)
            .from(playTrace)
            .innerJoin(score, eq(score.id, playTrace.scoreId))
            .leftJoin(playAnalysis, eq(playAnalysis.traceId, playTrace.id))

    const toTracedPlayRow = (row: Awaited<ReturnType<typeof selectTracedPlays>>[number]) =>
        ({
            traceId: row.traceId,
            scoreId: row.scoreId,
            chartSha256: row.chartSha256,
            chartMd5: row.chartMd5,
            playedAt: row.playedAt,
            score: {
                clear: row.clear,
                exScore: row.exScore,
                maxExScore: row.maxExScore,
                minbp: row.minbp,
                gauge: row.gauge,
                random: row.random,
                scratchAuto: row.scratchAuto,
                judgeRate: row.judgeRate,
                assist: row.assist,
                autoplay: row.autoplay,
                autoOffset: row.autoOffset,
                constant: row.constant,
                greenNumber: row.greenNumber,
            },
            analysis: row.analysisVersion === null ? null : { analysisVersion: row.analysisVersion, summary: row.summary },
        }) satisfies TracedPlayRow

    return {
        findOwnedScore: async ({ scoreId, userId }) => {
            const [row] = await db
                .select(scoreSelection)
                .from(score)
                .where(and(eq(score.id, scoreId), eq(score.userId, userId)))
                .limit(1)
            return row ?? null
        },
        findTraceIdByScoreId: async ({ scoreId, userId }) => {
            const [row] = await db
                .select({ id: playTrace.id })
                .from(playTrace)
                .where(and(eq(playTrace.scoreId, scoreId), eq(playTrace.userId, userId)))
                .limit(1)
            return row?.id ?? null
        },
        insertTraceIfAbsent: withLockRetry(async (row) => {
            try {
                await db.insert(playTrace).values(row)
                return true
            } catch (error) {
                if (hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])) return false
                throw error
            }
        }),
        sumUserTraces: async (userId) => {
            const [row] = await db
                .select({ count: count(), bytes: sum(playTrace.byteSize) })
                .from(playTrace)
                .where(eq(playTrace.userId, userId))
            return { count: row?.count ?? 0, bytes: Number(row?.bytes ?? 0) }
        },
        saveAnalysis: withLockRetry(async (row) => {
            const { analysisVersion, summary, patternJoin, patternMd5, computedAt } = row
            await db.insert(playAnalysis).values(row).onDuplicateKeyUpdate({ set: { analysisVersion, summary, patternJoin, patternMd5, computedAt } })
        }),
        saveAnalysisFailure: withLockRetry(async (row) => {
            const { analysisVersion, summary, computedAt } = row
            const failure = { analysisVersion, summary, patternJoin: ANALYSIS_FAILED_JOIN, patternMd5: null, computedAt }
            await db
                .insert(playAnalysis)
                .values({ traceId: row.traceId, ...failure })
                .onDuplicateKeyUpdate({ set: failure })
        }),
        findStoredTrace: async ({ traceId, userId }) => {
            const [row] = await db
                .select({
                    data: playTrace.data,
                    chartSha256: playTrace.chartSha256,
                    analysisVersion: playAnalysis.analysisVersion,
                    summary: playAnalysis.summary,
                })
                .from(playTrace)
                .leftJoin(playAnalysis, eq(playAnalysis.traceId, playTrace.id))
                .where(and(eq(playTrace.id, traceId), eq(playTrace.userId, userId)))
                .limit(1)
            if (!row) return null
            const analysis = row.analysisVersion === null ? null : { analysisVersion: row.analysisVersion, summary: row.summary }
            return { data: row.data, chartSha256: row.chartSha256, analysis }
        },
        summarizeUsage: async (limit) => {
            const [top, [totals]] = await Promise.all([
                db
                    .select({ loginId: user.loginId, ...traceTotals })
                    .from(playTrace)
                    .innerJoin(user, eq(user.id, playTrace.userId))
                    .groupBy(playTrace.userId, user.loginId)
                    .orderBy(desc(sum(playTrace.byteSize)))
                    .limit(limit),
                db.select({ users: countDistinct(playTrace.userId), ...traceTotals }).from(playTrace),
            ])
            return {
                top: top.map((row) => ({
                    loginId: row.loginId,
                    traces: row.traces,
                    bytes: Number(row.bytes ?? 0),
                    storedBytes: Number(row.storedBytes ?? 0),
                })),
                totals: {
                    users: totals?.users ?? 0,
                    traces: totals?.traces ?? 0,
                    bytes: Number(totals?.bytes ?? 0),
                    storedBytes: Number(totals?.storedBytes ?? 0),
                },
            }
        },
        countAccountsPast: async ({ near, over }) => {
            const perUser = db
                .select({ traces: count().as('traces'), bytes: sum(playTrace.byteSize).as('bytes') })
                .from(playTrace)
                .groupBy(playTrace.userId)
                .as('per_user')
            const [row] = await db
                .select({
                    near: sql<number>`coalesce(sum(case when ${perUser.traces} >= ${near.count} or ${perUser.bytes} >= ${near.bytes} then 1 else 0 end), 0)`,
                    over: sql<number>`coalesce(sum(case when ${perUser.traces} > ${over.count} or ${perUser.bytes} > ${over.bytes} then 1 else 0 end), 0)`,
                })
                .from(perUser)
            return { near: Number(row?.near ?? 0), over: Number(row?.over ?? 0) }
        },
        countAnalysesBehind: async (versions) => {
            const [row] = await db
                .select({
                    missing: sql<number>`coalesce(sum(case when ${playAnalysis.traceId} is null then 1 else 0 end), 0)`,
                    stale: sql<number>`coalesce(sum(case when ${playAnalysis.traceId} is not null and ${isBehindVersions(versions)} then 1 else 0 end), 0)`,
                })
                .from(playTrace)
                .leftJoin(playAnalysis, eq(playAnalysis.traceId, playTrace.id))
            return { missing: Number(row?.missing ?? 0), stale: Number(row?.stale ?? 0) }
        },
        listTracesBehind: async ({ versions, after, limit }) =>
            db
                .select({ id: playTrace.id, userId: playTrace.userId })
                .from(playTrace)
                .leftJoin(playAnalysis, eq(playAnalysis.traceId, playTrace.id))
                .where(and(or(isNull(playAnalysis.traceId), isBehindVersions(versions)), after === null ? undefined : gt(playTrace.id, after)))
                .orderBy(asc(playTrace.id))
                .limit(limit),
        listRecentPlays: async ({ userId, limit }) => {
            const rows = await selectTracedPlays()
                .where(and(eq(playTrace.userId, userId), eq(score.autoplay, false)))
                .orderBy(desc(playTrace.playedAt), desc(playTrace.id))
                .limit(limit)
            return rows.map(toTracedPlayRow)
        },
        listPlaysOfScores: async ({ userId, scoreIds }) => {
            if (scoreIds.length === 0) return []
            const rows = await selectTracedPlays().where(and(eq(playTrace.userId, userId), inArray(playTrace.scoreId, [...scoreIds])))
            return rows.map(toTracedPlayRow)
        },
        findPlayOfScore: async ({ userId, scoreId }) => {
            const [row] = await selectTracedPlays()
                .where(and(eq(playTrace.userId, userId), eq(playTrace.scoreId, scoreId)))
                .limit(1)
            return row ? toTracedPlayRow(row) : null
        },
        findBestPlayOnChart: async ({ userId, chartSha256, scratchAuto, excludeScoreId }) => {
            const [row] = await selectTracedPlays()
                .where(
                    and(
                        eq(playTrace.chartSha256, chartSha256),
                        eq(playTrace.userId, userId),
                        eq(score.scratchAuto, scratchAuto),
                        eq(score.autoplay, false),
                        excludeScoreId === null ? undefined : ne(playTrace.scoreId, excludeScoreId),
                    ),
                )
                .orderBy(desc(score.exScore), desc(playTrace.playedAt), desc(playTrace.id))
                .limit(1)
            return row ? toTracedPlayRow(row) : null
        },
        isRivalOf: async ({ userId, rivalId }) => {
            const [row] = await db
                .select({ rivalId: rival.rivalId })
                .from(rival)
                .where(and(eq(rival.userId, userId), eq(rival.rivalId, rivalId)))
                .limit(1)
            return row !== undefined
        },
        findShareSetting: async (userId) => {
            const [row] = await db.select().from(playShareSetting).where(eq(playShareSetting.userId, userId)).limit(1)
            return row ?? null
        },
        saveShareSetting: withLockRetry(async (row) => {
            await db
                .insert(playShareSetting)
                .values(row)
                .onDuplicateKeyUpdate({ set: { rivalSegments: row.rivalSegments, updatedAt: row.updatedAt } })
        }),
    } satisfies TraceServiceDb
}

export const composeTrace = (db: Database, deps: { chartPatternService: Pick<ChartPatternService, 'readPattern'> }) => ({
    traceService: createTraceService({
        db: createTraceDb(db),
        newId,
        now: () => Date.now(),
        loadPattern: deps.chartPatternService.readPattern,
        reportAnalysisFailure: (error) => console.error('trace analysis failed', summarizeError(error)),
    }),
})
