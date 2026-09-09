import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { StatsSummaryPanel } from '@widgets/home/stats-summary-panel'
import { RecentActivityPanel } from '@widgets/home/recent-activity-panel'
import { activityRecentQueryOptions, statsSummaryQueryOptions } from '@entities/stats.query'
import { getQueryClient } from '@shared/utils/get-query-client'

const RECENT_LIMIT = 20

export const HomeContent = async () => {
    const queryClient = getQueryClient()
    await Promise.all([
        queryClient.prefetchQuery(statsSummaryQueryOptions()),
        queryClient.prefetchQuery(activityRecentQueryOptions({ limit: RECENT_LIMIT })),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <StatsSummaryPanel />
            <RecentActivityPanel />
        </HydrationBoundary>
    )
}
