import 'server-only'
import type { z } from 'zod'
import { compose } from '@server/compose'
import type { ChartLeaderboardQuery, ChartSearchQuery, FeedQuery, PlayerLeaderboardQuery, PlayerRecentQuery } from '@server/dto/fe'
import type { RankingQuery } from '@server/dto/score'
import { toPagination } from '@server/lib/api-response'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseQueryObject } from '@server/lib/parse-request'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'
import {
    getChartMetaCached,
    getChartRankingCached,
    getClientBuildsCached,
    getCourseMetaCached,
    getCourseRankingCached,
    getPlayerLeaderboardCached,
    getPlayerProfileCached,
    getPlayerRecentCached,
    getPlayerStatsCached,
    getRecentActivityCached,
    getReplayCached,
    getRivalsCached,
    getStatsSummaryCached,
    getTableCached,
    getTablesCached,
    resolveChartSha256Cached,
    searchChartsCached,
} from '@server/service/read-cache'

export const CHART_LEADERBOARD_LIMIT = 50

type QueryValue = string | number | boolean | undefined | null

/**
 * Validates the parameter object a page holds with the same schema that guards the HTTP route. The values are turned into the
 * strings a query string would have carried, and empty ones drop out the way they drop out of a URL.
 */
export const parseQueryParams = <T>(schema: z.ZodType<T>, params: Record<string, QueryValue>) =>
    parseQueryObject(
        Object.fromEntries(
            Object.entries(params).flatMap(([key, value]) => (value === undefined || value === null || value === '' ? [] : [[key, String(value)]])),
        ),
        schema,
    )

const requireUserId = async (playerId: string) => {
    const userId = await compose().findUserIdByLoginId(playerId)
    if (!userId) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
    return userId
}

export const readChartLeaderboard = async (hash: string, query: ChartLeaderboardQuery) => {
    if (classifyChartHash(hash) === 'unknown') throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
    const resolved = await resolveChartSha256Cached(hash)
    if (!resolved) return { chart: null, ranking: [] }
    const [chart, ranking] = await Promise.all([
        getChartMetaCached(hash),
        getChartRankingCached({
            chartSha256: resolved.sha256,
            limit: CHART_LEADERBOARD_LIMIT,
            page: 1,
            includeScratchAuto: query.include_scratch_auto,
        }),
    ])
    return { chart, ranking }
}

export const readChartSearch = async (query: ChartSearchQuery) => {
    const { items, total } = await searchChartsCached(query)
    return { data: items, pagination: toPagination({ page: query.page, limit: query.limit, total }) }
}

export const readRecentActivity = async (query: FeedQuery) => getRecentActivityCached(query.limit)

export const readStatsSummary = async () => getStatsSummaryCached()

export const readPlayerLeaderboard = async (query: PlayerLeaderboardQuery) => {
    const { items, total } = await getPlayerLeaderboardCached(query)
    return { data: items, pagination: toPagination({ page: query.page, limit: query.limit, total }) }
}

export const readPlayerProfile = async (playerId: string) => {
    const profile = await getPlayerProfileCached(playerId)
    if (!profile) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
    return profile
}

export const readPlayerRecent = async (playerId: string, query: PlayerRecentQuery) => {
    const { items, total } = await getPlayerRecentCached({
        loginId: playerId,
        userId: await requireUserId(playerId),
        page: query.page,
        limit: query.limit,
    })
    return { data: items, pagination: toPagination({ page: query.page, limit: query.limit, total }) }
}

export const readPlayerStats = async (playerId: string) => getPlayerStatsCached({ loginId: playerId, userId: await requireUserId(playerId) })

export const readPlayerRivals = async (playerId: string) => {
    const rivals = await getRivalsCached(playerId)
    if (!rivals) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
    return rivals
}

export const readCourseMeta = async (courseHash: string) => {
    const meta = await getCourseMetaCached(courseHash)
    if (!meta) throw createAppError(ERROR_CODE.IR_COURSE_NOT_FOUND, { courseHash })
    return meta
}

export const readCourseRanking = async (courseHash: string, query: RankingQuery) =>
    getCourseRankingCached({
        courseHash,
        limit: query.limit,
        page: query.page,
        rivalOf: query.rival_of,
        includeScratchAuto: query.include_scratch_auto,
    })

export const readReplay = async (replayId: string) => {
    const replay = await getReplayCached(replayId)
    if (!replay) throw createAppError(ERROR_CODE.IR_REPLAY_NOT_FOUND, { replayId })
    return replay
}

export const readTables = async () => getTablesCached()

export const readTable = async (tableId: string) => {
    const table = await getTableCached(tableId)
    if (!table) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND, { tableId })
    return table
}

export const readClientBuilds = async () => getClientBuildsCached()

export const readApiTokens = async (userId: string) => compose().feService.listTokens(userId)

export const readSettingKeys = async (userId: string) => compose().settingService.listKeys(userId)
