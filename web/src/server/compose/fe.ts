import { and, asc, count, desc, eq, gte, inArray, isNotNull, like, or, sql } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, chart, chartBest, score } from '@server/db/schema'
import { MIN_CLEAR_LAMP_ID } from '@server/dto/common'
import { createFeService, type ActivityRow, type FeServiceDeps, type LeaderboardRow } from '@server/service/domain/fe/fe.service'
import { deriveAssistGrade } from '@server/service/domain/score/assist-grade'
import type { TableTextLookup } from '@server/service/domain/chart/chart.service'
import { toChartRow } from '@server/compose/chart-row'
import { chartBestScope } from '@server/compose/best-scope'
import { toJudgeBreakdown } from '@server/compose/judge-row'
import type { ScoreRankingRow } from '@server/service/domain/score/score.service'

const activitySelection = {
    scoreId: score.id,
    loginId: user.loginId,
    playerName: user.name,
    clear: score.clear,
    gauge: score.gauge,
    exScore: score.exScore,
    minbp: score.minbp,
    maxCombo: score.maxCombo,
    playedAt: score.playedAt,
    scratchAuto: score.scratchAuto,
    chartSha256: score.chartSha256,
    chartMd5: chart.md5,
    title: chart.title,
    artist: chart.artist,
    level: chart.level,
}

const toActivityRow = ({
    gauge,
    ...row
}: {
    scoreId: string
    loginId: string | null
    playerName: string | null
    clear: number
    gauge: number
    exScore: number
    minbp: number
    maxCombo: number
    playedAt: number
    scratchAuto: boolean
    chartSha256: string
    chartMd5: string | null
    title: string | null
    artist: string | null
    level: number | null
}): ActivityRow => ({
    ...row,
    assistGrade: deriveAssistGrade({ clear: row.clear, gauge }),
    title: row.title ?? '',
    artist: row.artist ?? '',
})

const CHART_SEARCH_ORDER = {
    title: [asc(chart.title)],
    level: [asc(chart.level), asc(chart.title)],
    notes: [desc(chart.notes)],
    recent: [desc(chart.createdAt)],
} as const

const LIKE_WILDCARDS = /[\\%_]/g

const escapeLike = (value: string) => value.replace(LIKE_WILDCARDS, (character) => `\\${character}`)

export const composeFe = (
    db: Database,
    deps: { lookupTableText: TableTextLookup; searchTableMd5s: (query: string) => string[]; listDisplayedDans: FeServiceDeps['listDisplayedDans'] },
) => {
    const buildLeaderboardStats = (includeScratchAuto: boolean) => {
        const playStats = db
            .select({ userId: score.userId, plays: count().as('plays') })
            .from(score)
            .where(and(isNotNull(score.userId), includeScratchAuto ? undefined : eq(score.scratchAuto, false)))
            .groupBy(score.userId)
            .as('play_stats')

        const bestStats = db
            .select({
                userId: chartBest.userId,
                playedCharts: sql<string>`count(distinct ${chartBest.chartSha256})`.as('played_charts'),
                clearedCharts: sql<string>`count(distinct case when ${chartBest.clear} >= ${MIN_CLEAR_LAMP_ID} then ${chartBest.chartSha256} end)`.as(
                    'cleared_charts',
                ),
            })
            .from(chartBest)
            .where(includeScratchAuto ? undefined : eq(chartBest.scratchAuto, false))
            .groupBy(chartBest.userId)
            .as('best_stats')

        const plays = sql`coalesce(${playStats.plays}, 0)`.mapWith(Number)
        const cleared = sql`coalesce(${bestStats.clearedCharts}, 0)`.mapWith(Number)
        const played = sql`coalesce(${bestStats.playedCharts}, 0)`.mapWith(Number)

        return {
            playStats,
            bestStats,
            plays,
            cleared,
            played,
            order: {
                cleared: [desc(cleared), desc(played)],
                played: [desc(played), desc(cleared)],
                plays: [desc(plays), desc(cleared)],
            },
        }
    }

    const feService = createFeService({
        lookupTableText: deps.lookupTableText,
        searchTableMd5s: deps.searchTableMd5s,
        listDisplayedDans: deps.listDisplayedDans,
        db: {
            searchCharts: async (query, { extraMd5s }) => {
                const keyword = query.q ? `%${escapeLike(query.q)}%` : null
                const where = and(
                    keyword
                        ? or(
                              like(chart.title, keyword),
                              like(chart.artist, keyword),
                              like(chart.genre, keyword),
                              extraMd5s.length > 0 ? inArray(chart.md5, extraMd5s) : undefined,
                          )
                        : undefined,
                    query.mode ? eq(chart.mode, query.mode) : undefined,
                    query.level === undefined ? undefined : eq(chart.level, query.level),
                )
                const rows = await db
                    .select()
                    .from(chart)
                    .where(where)
                    .orderBy(...CHART_SEARCH_ORDER[query.sort])
                    .limit(query.limit)
                    .offset((query.page - 1) * query.limit)
                const [total] = await db.select({ value: count() }).from(chart).where(where)
                return { rows: rows.map(toChartRow), total: total?.value ?? 0 }
            },

            listRecentActivity: async (limit) => {
                const rows = await db
                    .select(activitySelection)
                    .from(score)
                    .innerJoin(user, eq(user.id, score.userId))
                    .leftJoin(chart, eq(chart.sha256, score.chartSha256))
                    .where(and(eq(score.ranked, true), eq(score.scratchAuto, false)))
                    .orderBy(desc(score.playedAt))
                    .limit(limit)
                return rows.map(toActivityRow)
            },

            listPlayerRecent: async ({ userId, page, limit }) => {
                const where = eq(score.userId, userId)
                const rows = await db
                    .select(activitySelection)
                    .from(score)
                    .innerJoin(user, eq(user.id, score.userId))
                    .leftJoin(chart, eq(chart.sha256, score.chartSha256))
                    .where(where)
                    .orderBy(desc(score.playedAt), desc(score.id))
                    .limit(limit)
                    .offset((page - 1) * limit)
                const [total] = await db.select({ value: count() }).from(score).where(where)
                return { rows: rows.map(toActivityRow), total: total?.value ?? 0 }
            },

            listPlayerLeaderboard: async (query): Promise<{ rows: LeaderboardRow[]; total: number }> => {
                const stats = buildLeaderboardStats(query.include_scratch_auto)
                const rows = await db
                    .select({
                        id: user.id,
                        loginId: user.loginId,
                        name: user.name,
                        rank: user.rank,
                        rankPoints: user.rankPoints,
                        totalPlays: user.totalPlays,
                        role: user.role,
                        plays: stats.plays,
                        clearedCharts: stats.cleared,
                        playedCharts: stats.played,
                    })
                    .from(user)
                    .leftJoin(stats.playStats, eq(stats.playStats.userId, user.id))
                    .leftJoin(stats.bestStats, eq(stats.bestStats.userId, user.id))
                    .orderBy(...stats.order[query.sort], asc(user.loginId))
                    .limit(query.limit)
                    .offset((query.page - 1) * query.limit)
                const [total] = await db.select({ value: count() }).from(user)
                return { rows, total: total?.value ?? 0 }
            },

            countLampsByUser: async (userId) => {
                const rows = await db
                    .select({ clear: chartBest.clear, assistGrade: chartBest.assistGrade, scratchAuto: chartBest.scratchAuto, value: count() })
                    .from(chartBest)
                    .where(and(eq(chartBest.userId, userId), chartBestScope(db, true)))
                    .groupBy(chartBest.clear, chartBest.assistGrade, chartBest.scratchAuto)
                return rows.map((row) => ({ clear: row.clear, assistGrade: row.assistGrade, scratchAuto: row.scratchAuto, count: row.value }))
            },

            summarise: async () => {
                const [charts] = await db.select({ value: count() }).from(chart)
                const [players] = await db.select({ value: count() }).from(user)
                const [scores] = await db.select({ value: count() }).from(score)
                return { charts: charts?.value ?? 0, players: players?.value ?? 0, scores: scores?.value ?? 0 }
            },

            listTokens: async (userId) => {
                const rows = await db
                    .select({ id: apiToken.id, label: apiToken.label, createdAt: apiToken.createdAt, lastUsedAt: apiToken.lastUsedAt })
                    .from(apiToken)
                    .where(and(eq(apiToken.userId, userId), eq(apiToken.revoked, false)))
                    .orderBy(desc(apiToken.createdAt))
                return rows.map((row) => ({ id: row.id, label: row.label, createdAt: row.createdAt.getTime(), lastUsedAt: row.lastUsedAt }))
            },

            deleteToken: async ({ userId, tokenId }) => {
                const [row] = await db
                    .select({ id: apiToken.id })
                    .from(apiToken)
                    .where(and(eq(apiToken.id, tokenId), eq(apiToken.userId, userId)))
                    .limit(1)
                if (!row) return false
                await db.update(apiToken).set({ revoked: true }).where(eq(apiToken.id, tokenId))
                return true
            },
        },
    })

    const listPlayerScoreRows = async (params: { userId: string; since?: number; mode?: string; limit: number; page: number }) => {
        const rows = await db
            .select({
                scoreId: score.id,
                loginId: user.loginId,
                playerName: user.name,
                clear: score.clear,
                gauge: score.gauge,
                exScore: score.exScore,
                maxCombo: score.maxCombo,
                minbp: score.minbp,
                playedAt: score.playedAt,
                lntype: score.lntype,
                option: score.option,
                totalNotes: score.notes,
                scratchAuto: score.scratchAuto,
                pgreat: score.pgreat,
                great: score.great,
                good: score.good,
                bad: score.bad,
                poor: score.poor,
                miss: score.miss,
                fast: score.fast,
                slow: score.slow,
                combobreak: score.combobreak,
                epg: score.epg,
                lpg: score.lpg,
                egr: score.egr,
                lgr: score.lgr,
                egd: score.egd,
                lgd: score.lgd,
                ebd: score.ebd,
                lbd: score.lbd,
                epr: score.epr,
                lpr: score.lpr,
                ems: score.ems,
                lms: score.lms,
                avgjudge: score.avgjudge,
                emptyPoor: score.emptyPoor,
            })
            .from(score)
            .innerJoin(user, eq(user.id, score.userId))
            .where(
                and(
                    eq(score.userId, params.userId),
                    params.since === undefined ? undefined : gte(score.playedAt, params.since),
                    params.mode ? eq(score.mode, params.mode) : undefined,
                ),
            )
            .orderBy(desc(score.playedAt))
            .limit(params.limit)
            .offset((params.page - 1) * params.limit)
        return rows.map((row): ScoreRankingRow => ({
            scoreId: row.scoreId,
            loginId: row.loginId,
            playerName: row.playerName,
            clear: row.clear,
            assistGrade: deriveAssistGrade(row),
            exScore: row.exScore,
            maxCombo: row.maxCombo,
            minbp: row.minbp,
            playedAt: row.playedAt,
            lntype: row.lntype,
            option: row.option,
            totalNotes: row.totalNotes,
            scratchAuto: row.scratchAuto,
            judge: toJudgeBreakdown(row),
        }))
    }

    const countPlayerScores = async (userId: string) => {
        const [row] = await db.select({ value: count() }).from(score).where(eq(score.userId, userId))
        return row?.value ?? 0
    }

    return { feService, listPlayerScoreRows, countPlayerScores }
}
