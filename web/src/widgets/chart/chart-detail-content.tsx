import { notFound } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { ChartLeaderboardPanel } from '@widgets/chart/chart-leaderboard-panel'
import { chartLeaderboardQueryOptions } from '@entities/chart.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'

export const ChartDetailContent = async ({ params }: { params: Promise<{ hash: string }> }) => {
    const { hash } = await params
    const queryClient = getQueryClient()
    const leaderboard = await fetchQueryOrNotFound(queryClient, chartLeaderboardQueryOptions(hash))
    if (leaderboard && !leaderboard.chart) notFound()

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <ChartLeaderboardPanel hash={hash} />
        </HydrationBoundary>
    )
}
