import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope } from '@shared/lib/fetch'
import type { DanDisplayRequest, MyDans } from '@entities/dan.type'

export const myDansQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.DAN.MINE,
        queryFn: () => fetchEnvelope<MyDans>(buildApiUrl('/fe/me/dans')),
    })

export const useMyDans = () => useQuery(myDansQueryOptions())

/** Chooses the dan of one system that others see; the profile and the ranking show it, so their cached reads are dropped too. */
export const useSetDanDisplay = (playerId: string) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: DanDisplayRequest) =>
            fetchEnvelope<MyDans>(buildApiUrl('/fe/me/dans/display'), {
                method: 'PUT',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify(input),
            }),
        onSuccess: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.DAN.ALL }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.PLAYER.PROFILE(playerId) }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.LEADERBOARD.ALL }),
            ]),
    })
}
