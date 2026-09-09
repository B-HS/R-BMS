import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { ChartSearchPanel } from '@widgets/search/chart-search-panel'
import { chartSearchQueryOptions } from '@entities/chart.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readNumberParam, readParam } from '@shared/lib/search-params'

const DEFAULT_LIMIT = 20

export const ChartSearchContent = async ({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) => {
    const resolved = await searchParams
    const params = {
        q: readParam(resolved, 'q'),
        mode: readParam(resolved, 'mode'),
        level: readParam(resolved, 'level'),
        sort: readParam(resolved, 'sort') ?? 'title',
        page: readNumberParam(resolved, 'page', 1),
        limit: DEFAULT_LIMIT,
    }

    const queryClient = getQueryClient()
    await queryClient.prefetchQuery(chartSearchQueryOptions(params))

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <ChartSearchPanel params={params} />
        </HydrationBoundary>
    )
}
