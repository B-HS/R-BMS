import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchEnvelopePage } from '@shared/lib/fetch'
import type { ChartLeaderboard, ChartSearchParams, ChartSearchRow } from '@entities/chart.type'

export const chartLeaderboardQueryOptions = (hash: string) =>
    queryOptions({
        queryKey: QUERY_KEY.CHART.RANKING(hash),
        queryFn: () => fetchEnvelope<ChartLeaderboard>(buildApiUrl(`/fe/charts/${encodeURIComponent(hash)}/leaderboard`)),
    })

export const chartSearchQueryOptions = (params: ChartSearchParams) =>
    queryOptions({
        queryKey: QUERY_KEY.CHART.SEARCH(params),
        queryFn: () => fetchEnvelopePage<ChartSearchRow[]>(buildApiUrl('/fe/charts/search', params)),
    })

export const useChartLeaderboard = (hash: string) => useQuery(chartLeaderboardQueryOptions(hash))

export const useChartSearch = (params: ChartSearchParams) => useQuery(chartSearchQueryOptions(params))
