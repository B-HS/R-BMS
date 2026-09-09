import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchRaw } from '@shared/lib/fetch'
import type { ReplayData, ReplayMeta } from '@entities/replay.type'

export const replayDetailQueryOptions = (replayId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.REPLAY.DETAIL(replayId),
        queryFn: () => fetchRaw<ReplayData>(buildApiUrl(`/replays/${encodeURIComponent(replayId)}`)),
    })

export const chartReplayListQueryOptions = (hash: string, params: { limit: number }) =>
    queryOptions({
        queryKey: QUERY_KEY.CHART.REPLAYS(hash, params),
        queryFn: () => fetchRaw<ReplayMeta[]>(buildApiUrl(`/charts/${encodeURIComponent(hash)}/replays`, params)),
    })

export const useReplayDetail = (replayId: string) => useQuery(replayDetailQueryOptions(replayId))

export const useChartReplayList = (hash: string, params: { limit: number }) => useQuery(chartReplayListQueryOptions(hash, params))
