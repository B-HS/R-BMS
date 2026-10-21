import { MIN_CLEAR_LAMP_ID, clearLampFromId, type AssistGrade } from '@server/dto/common'
import { fillBlankText, toChartMeta, type ChartRow, type TableTextLookup } from '@server/service/domain/chart/chart.service'
import { toPlayerProfile, type PlayerRow } from '@server/service/domain/player/player.service'
import type { ChartSearchQuery, PlayerLeaderboardQuery } from '@server/dto/fe'

export type ActivityRow = {
    scoreId: string
    loginId: string | null
    playerName: string | null
    clear: number
    assistGrade: AssistGrade
    exScore: number
    minbp: number
    maxCombo: number
    playedAt: number
    scratchAuto: boolean
    chartSha256: string
    chartMd5: string | null
    title: string
    artist: string
    level: number | null
}

export type LampCountRow = {
    clear: number
    assistGrade: AssistGrade
    scratchAuto: boolean
    count: number
}

export type LeaderboardRow = PlayerRow & { plays: number; clearedCharts: number; playedCharts: number }

export type TokenRow = {
    id: string
    label: string
    createdAt: number
    lastUsedAt: number | null
}

export type FeServiceDb = {
    searchCharts: (query: ChartSearchQuery, options: { extraMd5s: string[] }) => Promise<{ rows: ChartRow[]; total: number }>
    listRecentActivity: (limit: number) => Promise<ActivityRow[]>
    listPlayerRecent: (params: { userId: string; page: number; limit: number }) => Promise<{ rows: ActivityRow[]; total: number }>
    listPlayerLeaderboard: (query: PlayerLeaderboardQuery) => Promise<{ rows: LeaderboardRow[]; total: number }>
    countLampsByUser: (userId: string) => Promise<LampCountRow[]>
    summarise: () => Promise<{ charts: number; players: number; scores: number }>
    listTokens: (userId: string) => Promise<TokenRow[]>
    deleteToken: (params: { userId: string; tokenId: string }) => Promise<boolean>
}

const buildActivityItem = (row: ActivityRow) => ({
    score_id: row.scoreId,
    player: { id: row.loginId ?? '' },
    player_name: row.playerName ?? '',
    clear: clearLampFromId(row.clear),
    assist_grade: row.assistGrade,
    ex_score: row.exScore,
    minbp: row.minbp,
    max_combo: row.maxCombo,
    played_at: row.playedAt,
    scratch_auto: row.scratchAuto,
    chart: { md5: row.chartMd5 ?? '', sha256: row.chartSha256, title: row.title, artist: row.artist, level: row.level },
})

export const toActivityItem = (row: ActivityRow, lookupTableText?: TableTextLookup) => {
    const text = fillBlankText({ md5: row.chartMd5 ?? '', title: row.title, artist: row.artist }, lookupTableText)
    return buildActivityItem({ ...row, title: text.title, artist: text.artist })
}

export const toLampDistribution = (rows: LampCountRow[]) => {
    const counts = new Map<string, Omit<LampCountRow, 'scratchAuto'>>()
    for (const row of rows) {
        const key = `${row.clear}:${row.assistGrade}`
        counts.set(key, { clear: row.clear, assistGrade: row.assistGrade, count: (counts.get(key)?.count ?? 0) + row.count })
    }
    return [...counts.values()].map((entry) => ({ clear: clearLampFromId(entry.clear), assist_grade: entry.assistGrade, count: entry.count }))
}

export const toLeaderboardItem = (row: LeaderboardRow) => ({
    ...toPlayerProfile(row, row.plays),
    cleared_charts: row.clearedCharts,
    played_charts: row.playedCharts,
})

export const createFeService = (deps: { db: FeServiceDb; lookupTableText?: TableTextLookup; searchTableMd5s?: (query: string) => string[] }) => ({
    searchCharts: async (query: ChartSearchQuery) => {
        const extraMd5s = query.q && deps.searchTableMd5s ? deps.searchTableMd5s(query.q) : []
        const { rows, total } = await deps.db.searchCharts(query, { extraMd5s })
        return { items: rows.map((row) => fillBlankText(toChartMeta(row), deps.lookupTableText)), total }
    },

    recentActivity: async (limit: number) => (await deps.db.listRecentActivity(limit)).map((row) => toActivityItem(row, deps.lookupTableText)),

    playerRecent: async (params: { userId: string; page: number; limit: number }) => {
        const { rows, total } = await deps.db.listPlayerRecent(params)
        return { items: rows.map((row) => toActivityItem(row, deps.lookupTableText)), total }
    },

    playerLeaderboard: async (query: PlayerLeaderboardQuery) => {
        const { rows, total } = await deps.db.listPlayerLeaderboard(query)
        return { items: rows.map(toLeaderboardItem), total }
    },

    playerStats: async (params: { userId: string; totalPlays: number }) => {
        const lamps = await deps.db.countLampsByUser(params.userId)
        return {
            total_plays: params.totalPlays,
            cleared: lamps.filter((row) => row.clear >= MIN_CLEAR_LAMP_ID).reduce((sum, row) => sum + row.count, 0),
            lamps: toLampDistribution(lamps),
            scratch_auto_charts: lamps.filter((row) => row.scratchAuto).reduce((sum, row) => sum + row.count, 0),
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
