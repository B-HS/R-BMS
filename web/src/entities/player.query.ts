import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchRaw } from '@shared/lib/fetch'
import type { PlayerProfile, PlayerRecentRow, PlayerStats } from '@entities/player.type'

export const playerQueryOptions = (playerId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.PROFILE(playerId),
        queryFn: () => fetchRaw<PlayerProfile>(buildApiUrl(`/players/${encodeURIComponent(playerId)}`)),
    })

export const playerRecentQueryOptions = (playerId: string, params: { limit: number }) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.RECENT(playerId, params),
        queryFn: () => fetchEnvelope<PlayerRecentRow[]>(buildApiUrl(`/fe/players/${encodeURIComponent(playerId)}/recent`, params)),
    })

export const playerStatsQueryOptions = (playerId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.STATS(playerId),
        queryFn: () => fetchEnvelope<PlayerStats>(buildApiUrl(`/fe/players/${encodeURIComponent(playerId)}/stats`)),
    })

export const usePlayer = (playerId: string) => useQuery(playerQueryOptions(playerId))

export const usePlayerRecent = (playerId: string, params: { limit: number }) => useQuery(playerRecentQueryOptions(playerId, params))

export const usePlayerStats = (playerId: string) => useQuery(playerStatsQueryOptions(playerId))
