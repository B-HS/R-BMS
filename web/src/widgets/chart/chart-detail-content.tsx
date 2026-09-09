import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { ChartLeaderboardPanel } from '@widgets/chart/chart-leaderboard-panel'
import { chartLeaderboardQueryOptions } from '@entities/chart.query'
import { getQueryClient } from '@shared/utils/get-query-client'

export const ChartDetailContent = async ({ params }: { params: Promise<{ hash: string }> }) => {
    const { hash } = await params
    const queryClient = getQueryClient()
    await queryClient.prefetchQuery(chartLeaderboardQueryOptions(hash))

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <ChartLeaderboardPanel hash={hash} />
        </HydrationBoundary>
    )
}
