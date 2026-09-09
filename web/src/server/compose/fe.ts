import { and, asc, count, desc, eq, gte, inArray, like, or } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { apiToken, chart, chartBest, score } from '@server/db/schema'
import { createFeService, type ActivityRow } from '@server/service/domain/fe/fe.service'
import { toChartRow } from '@server/compose/chart-row'
import type { PlayerRow } from '@server/service/domain/player/player.service'
import type { ScoreRankingRow } from '@server/service/domain/score/score.service'

const activitySelection = {
    scoreId: score.id,
    loginId: user.loginId,
    playerName: user.name,
    clear: score.clear,
    exScore: score.exScore,
    minbp: score.minbp,
    maxCombo: score.maxCombo,
    playedAt: score.playedAt,
    chartSha256: score.chartSha256,
    chartMd5: chart.md5,
    title: chart.title,
    artist: chart.artist,
    level: chart.level,
}

const toActivityRow = (row: {
    scoreId: string
    loginId: string | null
    playerName: string | null
    clear: number
    exScore: number
    minbp: number
    maxCombo: number
    playedAt: number
    chartSha256: string
    chartMd5: string | null
    title: string | null
    artist: string | null
    level: number | null
}): ActivityRow => ({
    ...row,
    title: row.title ?? '',
    artist: row.artist ?? '',
})

const CHART_SEARCH_ORDER = {
    title: [asc(chart.title)],
    level: [asc(chart.level), asc(chart.title)],
    notes: [desc(chart.notes)],
    recent: [desc(chart.createdAt)],
} as const

export const composeFe = (db: Database) => {
    const countPlaysFor = async (userIds: string[]) => {
        if (userIds.length === 0) return new Map<string, number>()
        const rows = await db.select({ userId: score.userId, value: count() }).from(score).where(inArray(score.userId, userIds)).groupBy(score.userId)
        return new Map(rows.map((row) => [row.userId ?? '', row.value]))
    }

    const feService = createFeService({
        db: {
            searchCharts: async (query) => {
                const keyword = query.q ? `%${query.q}%` : null
                const where = and(
                    keyword ? or(like(chart.title, keyword), like(chart.artist, keyword), like(chart.genre, keyword)) : undefined,
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
                    .where(eq(score.ranked, true))
                    .orderBy(desc(score.playedAt))
                    .limit(limit)
                return rows.map(toActivityRow)
            },

            listPlayerRecent: async ({ userId, limit }) => {
                const rows = await db
                    .select(activitySelection)
                    .from(score)
                    .innerJoin(user, eq(user.id, score.userId))
                    .leftJoin(chart, eq(chart.sha256, score.chartSha256))
                    .where(eq(score.userId, userId))
                    .orderBy(desc(score.playedAt))
                    .limit(limit)
                return rows.map(toActivityRow)
            },

            listPlayerLeaderboard: async (query) => {
                const rows = await db
                    .select({
                        id: user.id,
                        loginId: user.loginId,
                        name: user.name,
                        rank: user.rank,
                        rankPoints: user.rankPoints,
                        totalPlays: user.totalPlays,
                        role: user.role,
                    })
                    .from(user)
                    .orderBy(query.sort === 'total_plays' ? desc(user.totalPlays) : desc(user.rankPoints), asc(user.loginId))
                    .limit(query.limit)
                    .offset((query.page - 1) * query.limit)
                const plays = await countPlaysFor(rows.map((row: PlayerRow) => row.id))
                const [total] = await db.select({ value: count() }).from(user)
                return {
                    rows: rows.map((row: PlayerRow) => ({ ...row, plays: plays.get(row.id) ?? 0 })),
                    total: total?.value ?? 0,
                }
            },

            countLampsByUser: async (userId) => {
                const rows = await db
                    .select({ clear: chartBest.clear, value: count() })
                    .from(chartBest)
                    .where(eq(chartBest.userId, userId))
                    .groupBy(chartBest.clear)
                return rows.map((row) => ({ clear: row.clear, count: row.value }))
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
                exScore: score.exScore,
                maxCombo: score.maxCombo,
                minbp: score.minbp,
                playedAt: score.playedAt,
                lntype: score.lntype,
                option: score.option,
                totalNotes: score.notes,
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
            exScore: row.exScore,
            maxCombo: row.maxCombo,
            minbp: row.minbp,
            playedAt: row.playedAt,
            lntype: row.lntype,
            option: row.option,
            totalNotes: row.totalNotes,
            judge: {
                pgreat: row.pgreat,
                great: row.great,
                good: row.good,
                bad: row.bad,
                poor: row.poor,
                miss: row.miss,
                fast: row.fast,
                slow: row.slow,
                combobreak: row.combobreak,
                epg: row.epg,
                lpg: row.lpg,
                egr: row.egr,
                lgr: row.lgr,
                egd: row.egd,
                lgd: row.lgd,
                ebd: row.ebd,
                lbd: row.lbd,
                epr: row.epr,
                lpr: row.lpr,
                ems: row.ems,
                lms: row.lms,
                avgjudge: row.avgjudge,
                empty_poor: row.emptyPoor,
            },
        }))
    }

    const countPlayerScores = async (userId: string) => {
        const [row] = await db.select({ value: count() }).from(score).where(eq(score.userId, userId))
        return row?.value ?? 0
    }

    return { feService, listPlayerScoreRows, countPlayerScores }
}
