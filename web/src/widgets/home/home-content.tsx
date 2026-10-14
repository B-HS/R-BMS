import { connection } from 'next/server'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { RecentActivityPanel } from '@widgets/home/recent-activity-panel'
import { StatsSummaryPanel } from '@widgets/home/stats-summary-panel'
import { activityRecentQueryOptions, statsSummaryQueryOptions } from '@entities/stats.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { feedQuerySchema } from '@server/dto/fe'
import { parseQueryParams, readRecentActivity, readStatsSummary } from '@server/service/public-reads'

const RECENT_LIMIT = 20

export const HomeContent = async () => {
    await connection()
    const queryClient = getQueryClient()
    await Promise.all([
        queryClient.prefetchQuery({ ...statsSummaryQueryOptions(), queryFn: readStatsSummary }),
        queryClient.prefetchQuery({
            ...activityRecentQueryOptions({ limit: RECENT_LIMIT }),
            queryFn: () => readRecentActivity(parseQueryParams(feedQuerySchema, { limit: RECENT_LIMIT })),
        }),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <StatsSummaryPanel />
            <RecentActivityPanel />
        </HydrationBoundary>
    )
}
