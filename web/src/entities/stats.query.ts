import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchEnvelopePage } from '@shared/lib/fetch'
import type { ActivityRow, PlayerLeaderboardRow, StatsSummary } from '@entities/stats.type'

export const statsSummaryQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.STATS.SUMMARY,
        queryFn: () => fetchEnvelope<StatsSummary>(buildApiUrl('/fe/stats/summary')),
    })

export const activityRecentQueryOptions = (params: { limit: number }) =>
    queryOptions({
        queryKey: QUERY_KEY.ACTIVITY.RECENT(params),
        queryFn: () => fetchEnvelope<ActivityRow[]>(buildApiUrl('/fe/activity/recent', params)),
    })

export const playerLeaderboardQueryOptions = (params: { sort: string; page: number; limit: number }) =>
    queryOptions({
        queryKey: QUERY_KEY.LEADERBOARD.PLAYERS(params),
        queryFn: () => fetchEnvelopePage<PlayerLeaderboardRow[]>(buildApiUrl('/fe/leaderboards/players', params)),
    })

export const useStatsSummary = () => useQuery(statsSummaryQueryOptions())

export const useActivityRecent = (params: { limit: number }) => useQuery(activityRecentQueryOptions(params))

export const usePlayerLeaderboard = (params: { sort: string; page: number; limit: number }) => useQuery(playerLeaderboardQueryOptions(params))
