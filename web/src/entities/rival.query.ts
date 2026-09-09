import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchRaw } from '@shared/lib/fetch'
import type { PlayerProfile } from '@entities/player.type'

export const rivalListQueryOptions = (playerId: string, init?: RequestInit) =>
    queryOptions({
        queryKey: QUERY_KEY.PLAYER.RIVALS(playerId),
        queryFn: () => fetchRaw<PlayerProfile[]>(buildApiUrl(`/players/${encodeURIComponent(playerId)}/rivals`), init),
    })

export const useRivalList = (playerId: string) => useQuery(rivalListQueryOptions(playerId))

export const useSaveRivals = (playerId: string) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (rivals: string[]) =>
            fetchRaw<PlayerProfile[]>(buildApiUrl(`/players/${encodeURIComponent(playerId)}/rivals`), {
                method: 'PUT',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({ rivals }),
            }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.PLAYER.RIVALS(playerId) }),
    })
}
