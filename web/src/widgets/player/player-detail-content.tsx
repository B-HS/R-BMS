import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { PlayerDetailPanel } from '@widgets/player/player-detail-panel'
import { playerQueryOptions, playerRecentQueryOptions, playerStatsQueryOptions } from '@entities/player.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { readNumberParam } from '@shared/lib/search-params'
import { getQueryClient } from '@shared/utils/get-query-client'
import { playerRecentQuerySchema } from '@server/dto/fe'
import { parseQueryParams, readPlayerProfile, readPlayerRecent, readPlayerStats } from '@server/service/public-reads'

const RECENT_LIMIT = 20

type PlayerDetailContentProps = {
    params: Promise<{ playerId: string }>
    searchParams: Promise<Record<string, string | string[] | undefined>>
}

export const PlayerDetailContent = async ({ params, searchParams }: PlayerDetailContentProps) => {
    const { playerId } = await params
    const recentParams = { page: readNumberParam(await searchParams, 'page', 1), limit: RECENT_LIMIT }
    const queryClient = getQueryClient()
    await Promise.all([
        fetchQueryOrNotFound(queryClient, { ...playerQueryOptions(playerId), queryFn: () => readPlayerProfile(playerId) }),
        queryClient.prefetchQuery({ ...playerStatsQueryOptions(playerId), queryFn: () => readPlayerStats(playerId) }),
        queryClient.prefetchQuery({
            ...playerRecentQueryOptions(playerId, recentParams),
            queryFn: () => readPlayerRecent(playerId, parseQueryParams(playerRecentQuerySchema, recentParams)),
        }),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PlayerDetailPanel playerId={playerId} recentParams={recentParams} />
        </HydrationBoundary>
    )
}
