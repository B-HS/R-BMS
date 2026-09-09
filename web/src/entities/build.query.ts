import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope } from '@shared/lib/fetch'
import type { ClientBuild, ClientBuildInput } from '@entities/build.type'

export const clientBuildListQueryOptions = (init?: RequestInit) =>
    queryOptions({
        queryKey: QUERY_KEY.ADMIN.BUILDS,
        queryFn: () => fetchEnvelope<ClientBuild[]>(buildApiUrl('/admin/builds'), init),
    })

export const useClientBuildList = () => useQuery(clientBuildListQueryOptions())

export const useRegisterClientBuild = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: ClientBuildInput) =>
            fetchEnvelope<ClientBuild>(buildApiUrl('/admin/builds'), {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify(input),
            }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.ADMIN.BUILDS }),
    })
}
