import { and, count, countDistinct, desc, eq, sql, sum } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { playAnalysis, playTrace, score } from '@server/db/schema'
import { summarizeError } from '@server/lib/error-summary'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { newId } from '@server/lib/token'
import { withLockRetry } from '@server/lib/with-lock-retry'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { createTraceService, type TraceServiceDb } from '@server/service/domain/trace/trace.service'

const scoreSelection = {
    id: score.id,
    userId: score.userId,
    chartSha256: score.chartSha256,
    chartMd5: score.chartMd5,
    mode: score.mode,
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

const traceTotals = {
    traces: count(),
    bytes: sum(playTrace.byteSize),
    storedBytes: sum(playTrace.storedSize),
}

export const createTraceDb = (db: Database) =>
    ({
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
        countAnalysesBehind: async (version) => {
            const [row] = await db
                .select({
                    missing: sql<number>`coalesce(sum(case when ${playAnalysis.traceId} is null then 1 else 0 end), 0)`,
                    stale: sql<number>`coalesce(sum(case when ${playAnalysis.analysisVersion} < ${version} then 1 else 0 end), 0)`,
                })
                .from(playTrace)
                .leftJoin(playAnalysis, eq(playAnalysis.traceId, playTrace.id))
            return { missing: Number(row?.missing ?? 0), stale: Number(row?.stale ?? 0) }
        },
    }) satisfies TraceServiceDb

export const composeTrace = (db: Database, deps: { chartPatternService: Pick<ChartPatternService, 'readPattern'> }) => ({
    traceService: createTraceService({
        db: createTraceDb(db),
        newId,
        now: () => Date.now(),
        loadPattern: deps.chartPatternService.readPattern,
        reportAnalysisFailure: (error) => console.error('trace analysis failed', summarizeError(error)),
    }),
})
