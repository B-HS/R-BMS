import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { PlayerProfilePanel } from '@widgets/player/player-profile-panel'
import { PlayerStatsPanel } from '@widgets/player/player-stats-panel'
import { PlayerRecentPanel } from '@widgets/player/player-recent-panel'
import { playerQueryOptions, playerRecentQueryOptions, playerStatsQueryOptions } from '@entities/player.query'
import { getQueryClient } from '@shared/utils/get-query-client'

const RECENT_LIMIT = 20

export const PlayerDetailContent = async ({ params }: { params: Promise<{ playerId: string }> }) => {
    const { playerId } = await params
    const queryClient = getQueryClient()
    await Promise.all([
        queryClient.prefetchQuery(playerQueryOptions(playerId)),
        queryClient.prefetchQuery(playerStatsQueryOptions(playerId)),
        queryClient.prefetchQuery(playerRecentQueryOptions(playerId, { limit: RECENT_LIMIT })),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PlayerProfilePanel playerId={playerId} />
            <PlayerStatsPanel playerId={playerId} />
            <PlayerRecentPanel playerId={playerId} />
        </HydrationBoundary>
    )
}
