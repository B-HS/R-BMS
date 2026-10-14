import { notFound, unstable_rethrow } from 'next/navigation'
import type { FetchQueryOptions, QueryClient, QueryKey } from '@tanstack/react-query'
import { ApiError } from '@shared/lib/fetch'

const NOT_FOUND_STATUS = 404

/** Prefetches a query on the server and turns a 404 from the API into the route's not-found page; other failures are left for the client to retry. */
export const fetchQueryOrNotFound = async <TQueryFnData, TData, TQueryKey extends QueryKey>(
    queryClient: QueryClient,
    options: FetchQueryOptions<TQueryFnData, Error, TData, TQueryKey>,
) => {
    try {
        return await queryClient.fetchQuery(options)
    } catch (error) {
        unstable_rethrow(error)
        if (error instanceof ApiError && error.status === NOT_FOUND_STATUS) notFound()
        return null
    }
}
