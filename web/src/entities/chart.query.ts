import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchEnvelopePage } from '@shared/lib/fetch'
import type { ChartLeaderboard, ChartLeaderboardParams, ChartSearchParams, ChartSearchRow } from '@entities/chart.type'

export const chartLeaderboardQueryOptions = (hash: string, params: ChartLeaderboardParams) =>
    queryOptions({
        queryKey: QUERY_KEY.CHART.RANKING(hash, params),
        queryFn: () => fetchEnvelope<ChartLeaderboard>(buildApiUrl(`/fe/charts/${encodeURIComponent(hash)}/leaderboard`, params)),
    })

export const chartSearchQueryOptions = (params: ChartSearchParams) =>
    queryOptions({
        queryKey: QUERY_KEY.CHART.SEARCH(params),
        queryFn: () => fetchEnvelopePage<ChartSearchRow[]>(buildApiUrl('/fe/charts/search', params)),
    })

export const useChartLeaderboard = (hash: string, params: ChartLeaderboardParams) => useQuery(chartLeaderboardQueryOptions(hash, params))

export const useChartSearch = (params: ChartSearchParams) => useQuery(chartSearchQueryOptions(params))
