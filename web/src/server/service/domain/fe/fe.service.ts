import { clearLampFromId } from '@server/dto/common'
import { toChartMeta, type ChartRow } from '@server/service/domain/chart/chart.service'
import { toPlayerProfile, type PlayerRow } from '@server/service/domain/player/player.service'
import type { ChartSearchQuery, PlayerLeaderboardQuery } from '@server/dto/fe'

export type ActivityRow = {
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
    title: string
    artist: string
    level: number | null
}

export type LampCountRow = {
    clear: number
    count: number
}

export type TokenRow = {
    id: string
    label: string
    createdAt: number
    lastUsedAt: number | null
}

export type FeServiceDb = {
    searchCharts: (query: ChartSearchQuery) => Promise<{ rows: ChartRow[]; total: number }>
    listRecentActivity: (limit: number) => Promise<ActivityRow[]>
    listPlayerRecent: (params: { userId: string; limit: number }) => Promise<ActivityRow[]>
    listPlayerLeaderboard: (query: PlayerLeaderboardQuery) => Promise<{ rows: (PlayerRow & { plays: number })[]; total: number }>
    countLampsByUser: (userId: string) => Promise<LampCountRow[]>
    summarise: () => Promise<{ charts: number; players: number; scores: number }>
    listTokens: (userId: string) => Promise<TokenRow[]>
    deleteToken: (params: { userId: string; tokenId: string }) => Promise<boolean>
}

export const toActivityItem = (row: ActivityRow) => ({
    score_id: row.scoreId,
    player: { id: row.loginId ?? '' },
    player_name: row.playerName ?? '',
    clear: clearLampFromId(row.clear),
    ex_score: row.exScore,
    minbp: row.minbp,
    max_combo: row.maxCombo,
    played_at: row.playedAt,
    chart: { md5: row.chartMd5 ?? '', sha256: row.chartSha256, title: row.title, artist: row.artist, level: row.level },
})

export const toLampDistribution = (rows: LampCountRow[]) => rows.map((row) => ({ clear: clearLampFromId(row.clear), count: row.count }))

export const createFeService = (deps: { db: FeServiceDb }) => ({
    searchCharts: async (query: ChartSearchQuery) => {
        const { rows, total } = await deps.db.searchCharts(query)
        return { items: rows.map(toChartMeta), total }
    },

    recentActivity: async (limit: number) => (await deps.db.listRecentActivity(limit)).map(toActivityItem),

    playerRecent: async (params: { userId: string; limit: number }) => (await deps.db.listPlayerRecent(params)).map(toActivityItem),

    playerLeaderboard: async (query: PlayerLeaderboardQuery) => {
        const { rows, total } = await deps.db.listPlayerLeaderboard(query)
        return { items: rows.map((row) => toPlayerProfile(row, row.plays)), total }
    },

    playerStats: async (params: { userId: string; totalPlays: number }) => {
        const lamps = await deps.db.countLampsByUser(params.userId)
        return {
            total_plays: params.totalPlays,
            cleared: lamps.filter((row) => row.clear >= 4).reduce((sum, row) => sum + row.count, 0),
            lamps: toLampDistribution(lamps),
        }
    },

    summary: async () => {
        const summary = await deps.db.summarise()
        return { charts: summary.charts, players: summary.players, scores: summary.scores }
    },

    listTokens: async (userId: string) =>
        (await deps.db.listTokens(userId)).map((row) => ({
            id: row.id,
            label: row.label,
            created_at: row.createdAt,
            last_used_at: row.lastUsedAt,
        })),

    deleteToken: async (params: { userId: string; tokenId: string }) => deps.db.deleteToken(params),
})

export type FeService = ReturnType<typeof createFeService>
