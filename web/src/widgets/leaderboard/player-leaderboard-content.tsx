import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { PlayerLeaderboardPanel } from '@widgets/leaderboard/player-leaderboard-panel'
import { playerLeaderboardQueryOptions } from '@entities/stats.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readNumberParam, readParam } from '@shared/lib/search-params'

const DEFAULT_LIMIT = 20

export const PlayerLeaderboardContent = async ({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) => {
    const resolved = await searchParams
    const params = {
        sort: readParam(resolved, 'sort') ?? 'rank_points',
        page: readNumberParam(resolved, 'page', 1),
        limit: DEFAULT_LIMIT,
    }

    const queryClient = getQueryClient()
    await queryClient.prefetchQuery(playerLeaderboardQueryOptions(params))

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PlayerLeaderboardPanel params={params} />
        </HydrationBoundary>
    )
}
