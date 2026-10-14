import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { ReplayDetailPanel } from '@widgets/replay/replay-detail-panel'
import { replayDetailQueryOptions } from '@entities/replay.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'

export const ReplayDetailContent = async ({ params }: { params: Promise<{ replayId: string }> }) => {
    const { replayId } = await params

    const queryClient = getQueryClient()
    await fetchQueryOrNotFound(queryClient, replayDetailQueryOptions(replayId))

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <ReplayDetailPanel replayId={replayId} />
        </HydrationBoundary>
    )
}
