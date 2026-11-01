import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchEnvelopePage, fetchRaw } from '@shared/lib/fetch'
import type { PlayerPlayDays, PlayerProfile, PlayerRecentParams, PlayerRecentRow, PlayerStats } from '@entities/player.type'

export const playerQueryOptions = (playerId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.PROFILE(playerId),
        queryFn: () => fetchRaw<PlayerProfile>(buildApiUrl(`/players/${encodeURIComponent(playerId)}`)),
    })

export const playerRecentQueryOptions = (playerId: string, params: PlayerRecentParams) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.RECENT(playerId, params),
        queryFn: () => fetchEnvelopePage<PlayerRecentRow[]>(buildApiUrl(`/fe/players/${encodeURIComponent(playerId)}/recent`, params)),
    })

export const playerStatsQueryOptions = (playerId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.STATS(playerId),
        queryFn: () => fetchEnvelope<PlayerStats>(buildApiUrl(`/fe/players/${encodeURIComponent(playerId)}/stats`)),
    })

export const playerPlayDaysQueryOptions = (playerId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.PLAY_DAYS(playerId),
        queryFn: () => fetchEnvelope<PlayerPlayDays>(buildApiUrl(`/fe/players/${encodeURIComponent(playerId)}/play-days`)),
    })

export const usePlayer = (playerId: string) => useQuery(playerQueryOptions(playerId))

export const usePlayerRecent = (playerId: string, params: PlayerRecentParams) => useQuery(playerRecentQueryOptions(playerId, params))

export const usePlayerStats = (playerId: string) => useQuery(playerStatsQueryOptions(playerId))

export const usePlayerPlayDays = (playerId: string) => useQuery(playerPlayDaysQueryOptions(playerId))
