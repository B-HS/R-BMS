import { and, desc, eq, sql } from 'drizzle-orm'
import { alias } from 'drizzle-orm/mysql-core'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, playShareSetting, rival, score } from '@server/db/schema'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { createAiGaugeAdviceService, type AiGaugeAdviceServiceDb } from '@server/service/domain/ai/gauge-advice.service'
import { createAiPlayCompareService, type AiPlayCompareServiceDb } from '@server/service/domain/ai/play-compare.service'
import { createAiPlayReportService, type AiPlayReportServiceDb } from '@server/service/domain/ai/play-report.service'
import { createAiReplayCoachService, type PlayScoreDb } from '@server/service/domain/ai/replay-coach.service'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'
import type { TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import type { TraceService } from '@server/service/domain/trace/trace.service'
import { exScoreDenominator } from '@server/compose/ai-record-query'
import { createChartTagLoader, createLevelLabelOf, createTableLoaders } from '@server/compose/ai-score-features'

const recentScoreRows = (db: Database, userId: string, limit: number) =>
    db
        .select({ fast: score.fast, slow: score.slow, gauge: score.gauge, clear: score.clear })
        .from(score)
        .where(and(eq(score.userId, userId), eq(score.autoplay, false)))
        .orderBy(desc(score.playedAt), desc(score.id))
        .limit(limit)

export const createPlayScoreDb = (db: Database) =>
    ({
        findOwnedPlayScore: async ({ userId, scoreId }) => {
            const [row] = await db
                .select({
                    scoreId: score.id,
                    playedAt: score.playedAt,
                    chartSha256: score.chartSha256,
                    chartMd5: sql<string | null>`coalesce(${score.chartMd5}, ${chart.md5})`,
                    title: sql<string>`coalesce(${chart.title}, '')`,
                    artist: sql<string>`coalesce(${chart.artist}, '')`,
                    clear: score.clear,
                    exScore: score.exScore,
                    maxExScore: exScoreDenominator().mapWith(Number),
                    minbp: score.minbp,
                    random: score.random,
                    gauge: score.gauge,
                    scratchAuto: score.scratchAuto,
                    pgreat: score.pgreat,
                    great: score.great,
                    good: score.good,
                    bad: score.bad,
                    poor: score.poor,
                })
                .from(score)
                .leftJoin(chart, eq(chart.sha256, score.chartSha256))
                .where(and(eq(score.id, scoreId), eq(score.userId, userId)))
                .limit(1)
            return row ?? null
        },
    }) satisfies PlayScoreDb

export const createAiPlayCompareDb = (db: Database) =>
    ({
        ...createPlayScoreDb(db),
        findPublicBest: async ({ userId, chartSha256, scratchAuto }) => {
            const [row] = await db
                .select({
                    scoreId: chartBest.scoreId,
                    playedAt: score.playedAt,
                    clear: chartBest.clear,
                    gauge: score.gauge,
                    exScore: chartBest.exScore,
                    minbp: chartBest.minbp,
                    random: score.random,
                    scratchAuto: chartBest.scratchAuto,
                    pgreat: score.pgreat,
                    great: score.great,
                    good: score.good,
                    bad: score.bad,
                    poor: score.poor,
                })
                .from(chartBest)
                .innerJoin(score, eq(score.id, chartBest.scoreId))
                .where(and(eq(chartBest.chartSha256, chartSha256), eq(chartBest.userId, userId), eq(chartBest.scratchAuto, scratchAuto)))
                .limit(1)
            return row ?? null
        },
        listRivals: async (userId) => {
            const rivalKeepsUser = alias(rival, 'rival_keeps_user')
            const rows = await db
                .select({
                    userId: user.id,
                    loginId: user.loginId,
                    name: user.name,
                    isSharing: playShareSetting.rivalSegments,
                    keepsUser: rivalKeepsUser.userId,
                })
                .from(rival)
                .innerJoin(user, eq(user.id, rival.rivalId))
                .leftJoin(playShareSetting, eq(playShareSetting.userId, rival.rivalId))
                .leftJoin(rivalKeepsUser, and(eq(rivalKeepsUser.userId, rival.rivalId), eq(rivalKeepsUser.rivalId, rival.userId)))
                .where(eq(rival.userId, userId))
                .orderBy(user.loginId)
            return rows.map(({ userId: rivalId, loginId, name, isSharing, keepsUser }) => ({
                userId: rivalId,
                loginId,
                name,
                allowsSegments: isSharing === true && keepsUser !== null,
            }))
        },
        findLoginId: async (userId) => {
            const [row] = await db.select({ loginId: user.loginId }).from(user).where(eq(user.id, userId)).limit(1)
            return row?.loginId ?? null
        },
    }) satisfies AiPlayCompareServiceDb

export const createAiPlayReportDb = (db: Database) =>
    ({
        sumRecentFastSlow: async ({ userId, limit }) => {
            const rows = await recentScoreRows(db, userId, limit)
            return {
                plays: rows.length,
                fast: rows.reduce((sum, row) => sum + row.fast, 0),
                slow: rows.reduce((sum, row) => sum + row.slow, 0),
            }
        },
    }) satisfies AiPlayReportServiceDb

export const createAiGaugeAdviceDb = (db: Database) =>
    ({
        listRecentGaugeRows: async ({ userId, limit }) =>
            (await recentScoreRows(db, userId, limit)).map((row) => ({ gauge: row.gauge, clear: row.clear })),
    }) satisfies AiGaugeAdviceServiceDb

type ComposeAiPlayFeaturesDeps = {
    aiFeatureRunner: Pick<AiFeatureRunner, 'run'>
    aiResultService: Pick<AiResultService, 'findLatest'>
    traceService: Pick<TraceService, 'recentAnalyzedPlays' | 'analyzedPlayOfScore' | 'bestAnalyzedPlayOnChart' | 'rivalSegmentsOnChart'>
    tableSnapshotService: Pick<TableSnapshotService, 'getRecords'>
    chartPatternService: Pick<ChartPatternService, 'readFeaturesMany'>
    loadTables: () => Map<string, SnapshotTable>
}

/** The AI features that read play analyses (W5): the player report, the replay coach, the comparison and the gauge and option advice. */
export const composeAiPlayFeatures = (db: Database, deps: ComposeAiPlayFeaturesDeps) => {
    const shared = { runner: deps.aiFeatureRunner, results: deps.aiResultService, plays: deps.traceService }
    return {
        aiPlayReportService: createAiPlayReportService({
            ...shared,
            db: createAiPlayReportDb(db),
            ...createTableLoaders(deps),
            loadChartTags: createChartTagLoader(deps.chartPatternService),
        }),
        aiReplayCoachService: createAiReplayCoachService({ ...shared, db: createPlayScoreDb(db) }),
        aiPlayCompareService: createAiPlayCompareService({ ...shared, db: createAiPlayCompareDb(db) }),
        aiGaugeAdviceService: createAiGaugeAdviceService({
            ...shared,
            db: createAiGaugeAdviceDb(db),
            levelLabelOf: createLevelLabelOf(deps.loadTables),
        }),
    }
}
