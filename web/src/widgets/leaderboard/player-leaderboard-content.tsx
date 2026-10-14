import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { PlayerLeaderboardPanel } from '@widgets/leaderboard/player-leaderboard-panel'
import { playerLeaderboardQueryOptions } from '@entities/stats.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readBooleanParam, readNumberParam, readParam } from '@shared/lib/search-params'
import { DEFAULT_PLAYER_SORT } from '@shared/constants/leaderboard'
import { INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'
import { playerLeaderboardQuerySchema } from '@server/dto/fe'
import { parseQueryParams, readPlayerLeaderboard } from '@server/service/public-reads'

const DEFAULT_LIMIT = 20

export const PlayerLeaderboardContent = async ({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) => {
    const resolved = await searchParams
    const params = {
        sort: readParam(resolved, 'sort') ?? DEFAULT_PLAYER_SORT,
        page: readNumberParam(resolved, 'page', 1),
        limit: DEFAULT_LIMIT,
        include_scratch_auto: readBooleanParam(resolved, INCLUDE_SCRATCH_AUTO_PARAM),
    }

    const queryClient = getQueryClient()
    await queryClient.prefetchQuery({
        ...playerLeaderboardQueryOptions(params),
        queryFn: () => readPlayerLeaderboard(parseQueryParams(playerLeaderboardQuerySchema, params)),
    })

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PlayerLeaderboardPanel params={params} />
        </HydrationBoundary>
    )
}
