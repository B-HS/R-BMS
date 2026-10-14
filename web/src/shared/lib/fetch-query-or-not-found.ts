import { notFound, unstable_rethrow } from 'next/navigation'
import type { FetchQueryOptions, QueryClient, QueryKey } from '@tanstack/react-query'
import { ApiError } from '@shared/lib/fetch'

const NOT_FOUND_STATUS = 404

const isNotFoundError = (error: unknown) => {
    if (error instanceof ApiError) return error.status === NOT_FOUND_STATUS
    return typeof error === 'object' && error !== null && 'statusCode' in error && error.statusCode === NOT_FOUND_STATUS
}

/** Prefetches a query on the server and turns a not-found answer, from the API or from a direct server read, into the route's not-found page; other failures are left for the client to retry. */
export const fetchQueryOrNotFound = async <TQueryFnData, TData, TQueryKey extends QueryKey>(
    queryClient: QueryClient,
    options: FetchQueryOptions<TQueryFnData, Error, TData, TQueryKey>,
) => {
    try {
        return await queryClient.fetchQuery(options)
    } catch (error) {
        unstable_rethrow(error)
        if (isNotFoundError(error)) notFound()
        return null
    }
}
