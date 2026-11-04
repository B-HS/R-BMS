import { and, asc, count, desc, eq, gt, inArray, max, sql } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, chartBest, course, courseBest, courseScore, replay, rival, score } from '@server/db/schema'
import { MIN_CLEAR_LAMP_ID, clearLampToId } from '@server/dto/common'
import type { CourseCatalogOutput } from '@server/dto/course-catalog'
import type { TableTextLookup } from '@server/service/domain/chart/chart.service'
import { HARD_LAMP_ID, createDashboardService } from '@server/service/domain/dashboard/dashboard.service'
import type { TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { DEFAULT_INCLUDE_SCRATCH_AUTO } from '@shared/constants/scratch-auto'
import { chartBestScope, courseBestScope } from '@server/compose/best-scope'
import { activitySelection, buildLeaderboardStats, listPlayedAtsOf, toActivityRow } from '@server/compose/fe'

const FULL_COMBO_LAMP_ID = clearLampToId('FullCombo')

const countAtLeast = (lampId: number) => sql<string>`count(case when ${chartBest.clear} >= ${lampId} then 1 end)`.mapWith(Number)

export const composeDashboard = (
    db: Database,
    deps: {
        tableSnapshotService: Pick<TableSnapshotService, 'getDetail' | 'getRecords'>
        listCatalogGroups: () => Promise<CourseCatalogOutput>
        lookupTableText: TableTextLookup
    },
) => {
    const dashboardService = createDashboardService({
        tableSnapshotService: deps.tableSnapshotService,
        listCatalogGroups: deps.listCatalogGroups,
        lookupTableText: deps.lookupTableText,
        db: {
            countChartBests: async (userId) => {
                const [row] = await db
                    .select({
                        played: count(),
                        cleared: countAtLeast(MIN_CLEAR_LAMP_ID),
                        hardOrBetter: countAtLeast(HARD_LAMP_ID),
                        fullComboOrBetter: countAtLeast(FULL_COMBO_LAMP_ID),
                    })
                    .from(chartBest)
                    .where(and(eq(chartBest.userId, userId), chartBestScope(db, true)))
                return row ?? { played: 0, cleared: 0, hardOrBetter: 0, fullComboOrBetter: 0 }
            },

            listGradeInputs: async (userId) =>
                db
                    .select({ exScore: score.exScore, notes: score.notes, count: count() })
                    .from(chartBest)
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .where(and(eq(chartBest.userId, userId), chartBestScope(db, true), gt(score.notes, 0)))
                    .groupBy(score.exScore, score.notes),

            summariseScores: async (userId) => {
                const [row] = await db
                    .select({ total: count(), lastPlayedAt: max(score.playedAt) })
                    .from(score)
                    .where(eq(score.userId, userId))
                return { total: row?.total ?? 0, lastPlayedAt: row?.lastPlayedAt ?? null }
            },

            listPlayedAts: listPlayedAtsOf(db),

            listStandings: async () => {
                const stats = buildLeaderboardStats(db, DEFAULT_INCLUDE_SCRATCH_AUTO)
                return db
                    .select({
                        userId: stats.bestStats.userId,
                        position: sql<number>`rank() over (order by ${sql.join(stats.order.cleared, sql`, `)})`.mapWith(Number),
                    })
                    .from(stats.bestStats)
                    .innerJoin(user, eq(user.id, stats.bestStats.userId))
            },

            listCourseBests: async (userId) =>
                db
                    .select({ courseHash: courseBest.courseHash, clear: courseBest.clear })
                    .from(courseBest)
                    .where(and(eq(courseBest.userId, userId), courseBestScope(db, true))),

            listRecentCourseScores: async ({ userId, limit }) => {
                const rows = await db
                    .select({
                        courseHash: courseScore.courseHash,
                        name: course.name,
                        clear: courseScore.clear,
                        exScore: courseScore.exScore,
                        playedAt: courseScore.playedAt,
                    })
                    .from(courseScore)
                    .leftJoin(course, eq(course.courseHash, courseScore.courseHash))
                    .where(eq(courseScore.userId, userId))
                    .orderBy(desc(courseScore.playedAt), desc(courseScore.id))
                    .limit(limit)
                return rows.map((row) => ({ ...row, name: row.name ?? '' }))
            },

            listRivals: async (userId) =>
                db
                    .select({ userId: user.id, loginId: user.loginId, name: user.name })
                    .from(rival)
                    .innerJoin(user, eq(user.id, rival.rivalId))
                    .where(eq(rival.userId, userId))
                    .orderBy(asc(user.loginId)),

            listRecentRivalActivity: async ({ userIds, limit }) => {
                const rows = await db
                    .select(activitySelection)
                    .from(score)
                    .innerJoin(user, eq(user.id, score.userId))
                    .leftJoin(chart, eq(chart.sha256, score.chartSha256))
                    .where(and(inArray(score.userId, userIds), eq(score.ranked, true)))
                    .orderBy(desc(score.playedAt), desc(score.id))
                    .limit(limit)
                return rows.map(toActivityRow)
            },

            listVersus: async ({ userId, rivalIds }) => {
                const mine = db
                    .select({ chartSha256: chartBest.chartSha256, exScore: chartBest.exScore })
                    .from(chartBest)
                    .where(and(eq(chartBest.userId, userId), chartBestScope(db, true)))
                    .as('my_best')
                return db
                    .select({
                        rivalId: chartBest.userId,
                        common: count(),
                        wins: sql<string>`count(case when ${mine.exScore} > ${chartBest.exScore} then 1 end)`.mapWith(Number),
                        losses: sql<string>`count(case when ${mine.exScore} < ${chartBest.exScore} then 1 end)`.mapWith(Number),
                    })
                    .from(chartBest)
                    .innerJoin(mine, eq(mine.chartSha256, chartBest.chartSha256))
                    .where(and(inArray(chartBest.userId, rivalIds), chartBestScope(db, true)))
                    .groupBy(chartBest.userId)
            },

            listRecentReplays: async ({ userId, limit }) => {
                const rows = await db
                    .select({
                        id: replay.id,
                        createdAt: replay.createdAt,
                        chartSha256: replay.chartSha256,
                        chartMd5: chart.md5,
                        title: chart.title,
                        artist: chart.artist,
                        level: chart.level,
                        clear: score.clear,
                        exScore: score.exScore,
                    })
                    .from(replay)
                    .leftJoin(chart, eq(chart.sha256, replay.chartSha256))
                    .leftJoin(score, eq(score.id, replay.scoreId))
                    .where(eq(replay.userId, userId))
                    .orderBy(desc(replay.createdAt), desc(replay.id))
                    .limit(limit)
                return rows.map((row) => ({ ...row, createdAt: row.createdAt.getTime() }))
            },
        },
    })

    return { dashboardService }
}
