import 'server-only'
import { cacheLife, cacheTag, revalidateTag } from 'next/cache'
import { compose, type Composed } from '@server/compose'
import { toScoreRecord } from '@server/service/domain/score/score.service'

export const CACHE_TAG = {
    chart: (hash: string) => `chart:${hash}`,
    chartRanking: (sha256: string) => `chart-ranking:${sha256}`,
    chartBest: (sha256: string) => `chart-best:${sha256}`,
    chartSearch: () => 'chart-search',
    score: (scoreId: string) => `score:${scoreId}`,
    player: (loginId: string) => `player:${loginId}`,
    playerScores: (loginId: string) => `player-scores:${loginId}`,
    playerRecent: (loginId: string) => `player-recent:${loginId}`,
    playerStats: (loginId: string) => `player-stats:${loginId}`,
    activityRecent: () => 'activity-recent',
    statsSummary: () => 'stats-summary',
    leaderboardPlayers: () => 'leaderboard-players',
    clientBuilds: () => 'client-builds',
}

export const resolveChartSha256Cached = async (hash: string) => {
    'use cache'
    cacheLife('days')
    cacheTag(CACHE_TAG.chart(hash))
    const row = await compose().chartService.findByHash(hash)
    return row ? { sha256: row.sha256, md5: row.md5 } : null
}

export const getChartMetaCached = async (hash: string) => {
    'use cache'
    cacheLife('days')
    cacheTag(CACHE_TAG.chart(hash))
    return compose().chartService.getMetaByHash(hash)
}

export const getChartRankingCached = async (params: { chartSha256: string; limit: number; page: number; lnmode?: number; rivalOf?: string }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.chartRanking(params.chartSha256))
    return compose().scoreService.getRanking(params)
}

export const getChartBestCached = async (params: { chartSha256: string; loginId: string; lnmode?: number }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.chartBest(params.chartSha256))
    return compose().scoreService.getBest(params)
}

export const getScoreCached = async (scoreId: string) => {
    'use cache'
    cacheLife('max')
    cacheTag(CACHE_TAG.score(scoreId))
    return compose().scoreService.getById(scoreId)
}

export const getPlayerProfileCached = async (loginId: string) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.player(loginId))
    return compose().playerService.getProfileByLoginId(loginId)
}

export const CACHE_PROFILE = {
    chart: 'days',
    ranking: 'minutes',
    player: 'minutes',
    feed: 'minutes',
} as const

export const revalidateOnScoreSubmit = (params: { sha256: string; md5: string | null; loginId: string | null }) => {
    revalidateTag(CACHE_TAG.chart(params.sha256), CACHE_PROFILE.chart)
    if (params.md5) revalidateTag(CACHE_TAG.chart(params.md5), CACHE_PROFILE.chart)
    revalidateTag(CACHE_TAG.chartRanking(params.sha256), CACHE_PROFILE.ranking)
    revalidateTag(CACHE_TAG.chartBest(params.sha256), CACHE_PROFILE.ranking)
    revalidateTag(CACHE_TAG.activityRecent(), CACHE_PROFILE.feed)
    revalidateTag(CACHE_TAG.statsSummary(), CACHE_PROFILE.feed)
    revalidateTag(CACHE_TAG.leaderboardPlayers(), CACHE_PROFILE.feed)
    if (!params.loginId) return
    revalidateTag(CACHE_TAG.player(params.loginId), CACHE_PROFILE.player)
    revalidateTag(CACHE_TAG.playerScores(params.loginId), CACHE_PROFILE.player)
    revalidateTag(CACHE_TAG.playerRecent(params.loginId), CACHE_PROFILE.player)
    revalidateTag(CACHE_TAG.playerStats(params.loginId), CACHE_PROFILE.player)
}

export const revalidateOnChartUpsert = (params: { sha256: string; md5: string | null }) => {
    revalidateTag(CACHE_TAG.chart(params.sha256), CACHE_PROFILE.chart)
    if (params.md5) revalidateTag(CACHE_TAG.chart(params.md5), CACHE_PROFILE.chart)
    revalidateTag(CACHE_TAG.chartSearch(), CACHE_PROFILE.ranking)
}

export const CACHE_TAG_EXT = {
    chartReplays: (sha256: string) => `chart-replays:${sha256}`,
    replay: (replayId: string) => `replay:${replayId}`,
    course: (courseHash: string) => `course:${courseHash}`,
    courseRanking: (courseHash: string) => `course-ranking:${courseHash}`,
    courseBest: (courseHash: string) => `course-best:${courseHash}`,
    tables: () => 'tables',
    table: (tableId: string) => `table:${tableId}`,
    playerRivals: (loginId: string) => `player-rivals:${loginId}`,
}

export const getCourseMetaCached = async (courseHash: string) => {
    'use cache'
    cacheLife('days')
    cacheTag(CACHE_TAG_EXT.course(courseHash))
    return compose().courseService.getMeta(courseHash)
}

export const getCourseRankingCached = async (params: { courseHash: string; limit: number; page: number; rivalOf?: string }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG_EXT.courseRanking(params.courseHash))
    return compose().courseService.getRanking(params)
}

export const getCourseBestCached = async (params: { courseHash: string; loginId: string }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG_EXT.courseBest(params.courseHash))
    return compose().courseService.getBest(params)
}

export const getChartReplaysCached = async (params: { chartSha256: string; userId?: string; limit: number }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG_EXT.chartReplays(params.chartSha256))
    return compose().replayService.listByChart(params)
}

export const getReplayCached = async (replayId: string) => {
    'use cache'
    cacheLife('max')
    cacheTag(CACHE_TAG_EXT.replay(replayId))
    return compose().replayService.getById(replayId)
}

export const getTablesCached = async () => {
    'use cache'
    cacheLife('hours')
    cacheTag(CACHE_TAG_EXT.tables())
    return compose().tableService.list()
}

export const getTableCached = async (tableId: string) => {
    'use cache'
    cacheLife('hours')
    cacheTag(CACHE_TAG_EXT.table(tableId))
    return compose().tableService.getById(tableId)
}

export const getRivalsCached = async (loginId: string) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG_EXT.playerRivals(loginId))
    return compose().rivalService.listByLoginId(loginId)
}

export const getClientBuildsCached = async () => {
    'use cache'
    cacheLife('hours')
    cacheTag(CACHE_TAG.clientBuilds())
    return compose().buildService.list()
}

export const searchChartsCached = async (query: Parameters<Composed['feService']['searchCharts']>[0]) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.chartSearch())
    return compose().feService.searchCharts(query)
}

export const getRecentActivityCached = async (limit: number) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.activityRecent())
    return compose().feService.recentActivity(limit)
}

export const getPlayerLeaderboardCached = async (query: Parameters<Composed['feService']['playerLeaderboard']>[0]) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.leaderboardPlayers())
    return compose().feService.playerLeaderboard(query)
}

export const getStatsSummaryCached = async () => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.statsSummary())
    return compose().feService.summary()
}

export const revalidateOnReplayUpload = (params: { chartSha256: string; loginId: string | null; scoreId?: string | null }) => {
    revalidateTag(CACHE_TAG_EXT.chartReplays(params.chartSha256), CACHE_PROFILE.ranking)
    if (params.scoreId) revalidateTag(CACHE_TAG.score(params.scoreId), CACHE_PROFILE.ranking)
    if (params.loginId) revalidateTag(CACHE_TAG.playerRecent(params.loginId), CACHE_PROFILE.player)
}

export const revalidateOnCourseSubmit = (params: { courseHash: string; loginId: string | null }) => {
    revalidateTag(CACHE_TAG_EXT.courseRanking(params.courseHash), CACHE_PROFILE.ranking)
    revalidateTag(CACHE_TAG_EXT.courseBest(params.courseHash), CACHE_PROFILE.ranking)
    if (params.loginId) revalidateTag(CACHE_TAG.playerStats(params.loginId), CACHE_PROFILE.player)
}

export const revalidateOnCourseUpsert = (courseHash: string) => {
    revalidateTag(CACHE_TAG_EXT.course(courseHash), CACHE_PROFILE.chart)
    revalidateTag(CACHE_TAG_EXT.tables(), CACHE_PROFILE.chart)
}

export const revalidateOnTableUpsert = (tableId: string) => {
    revalidateTag(CACHE_TAG_EXT.table(tableId), CACHE_PROFILE.chart)
    revalidateTag(CACHE_TAG_EXT.tables(), CACHE_PROFILE.chart)
}

export const revalidateOnRivalChange = (loginId: string) => {
    revalidateTag(CACHE_TAG_EXT.playerRivals(loginId), CACHE_PROFILE.player)
}

export const revalidateOnBuildUpsert = () => {
    revalidateTag(CACHE_TAG.clientBuilds(), CACHE_PROFILE.chart)
}

export const getPlayerScoresCached = async (params: {
    loginId: string
    userId: string
    since?: number
    mode?: string
    limit: number
    page: number
}) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.playerScores(params.loginId))
    const rows = await compose().listPlayerScoreRows(params)
    return rows.map((row) => toScoreRecord(row, null))
}

export const getPlayerRecentCached = async (params: { loginId: string; userId: string; limit: number }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.playerRecent(params.loginId))
    return compose().feService.playerRecent(params)
}

export const getPlayerStatsCached = async (params: { loginId: string; userId: string }) => {
    'use cache'
    cacheLife('minutes')
    cacheTag(CACHE_TAG.playerStats(params.loginId))
    const totalPlays = await compose().countPlayerScores(params.userId)
    return compose().feService.playerStats({ userId: params.userId, totalPlays })
}
