import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchNoContent } from '@shared/lib/fetch'
import type { ApiTokenRow, IssuedApiToken } from '@entities/token.type'

export const apiTokenListQueryOptions = (init?: RequestInit) =>
    queryOptions({
        queryKey: QUERY_KEY.TOKEN.LIST,
        queryFn: () => fetchEnvelope<ApiTokenRow[]>(buildApiUrl('/fe/tokens'), init),
    })

export const useApiTokenList = () => useQuery(apiTokenListQueryOptions())

export const useIssueApiToken = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: { label: string }) =>
            fetchEnvelope<IssuedApiToken>(buildApiUrl('/fe/tokens'), {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify(input),
            }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.TOKEN.ALL }),
    })
}

export const useRevokeApiToken = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (tokenId: string) => fetchNoContent(buildApiUrl(`/fe/tokens/${encodeURIComponent(tokenId)}`), { method: 'DELETE' }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.TOKEN.ALL }),
    })
}
