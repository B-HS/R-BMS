import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { PlayerDetailPanel } from '@widgets/player/player-detail-panel'
import { playerQueryOptions, playerRecentQueryOptions, playerStatsQueryOptions } from '@entities/player.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'
import { feedQuerySchema } from '@server/dto/fe'
import { parseQueryParams, readPlayerProfile, readPlayerRecent, readPlayerStats } from '@server/service/public-reads'

const RECENT_LIMIT = 20

export const PlayerDetailContent = async ({ params }: { params: Promise<{ playerId: string }> }) => {
    const { playerId } = await params
    const queryClient = getQueryClient()
    await Promise.all([
        fetchQueryOrNotFound(queryClient, { ...playerQueryOptions(playerId), queryFn: () => readPlayerProfile(playerId) }),
        queryClient.prefetchQuery({ ...playerStatsQueryOptions(playerId), queryFn: () => readPlayerStats(playerId) }),
        queryClient.prefetchQuery({
            ...playerRecentQueryOptions(playerId, { limit: RECENT_LIMIT }),
            queryFn: () => readPlayerRecent(playerId, parseQueryParams(feedQuerySchema, { limit: RECENT_LIMIT })),
        }),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PlayerDetailPanel playerId={playerId} />
        </HydrationBoundary>
    )
}
