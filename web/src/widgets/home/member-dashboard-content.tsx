import 'server-only'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { MemberDashboardGrid } from '@widgets/home/member-dashboard-grid'
import { dashboardOverviewQueryOptions } from '@entities/dashboard.query'
import { playerPlayDaysQueryOptions, playerQueryOptions, playerRecentQueryOptions, playerStatsQueryOptions } from '@entities/player.query'
import type { SessionUser } from '@entities/auth.type'
import { DASHBOARD_RECENT_PLAYS_LIMIT } from '@shared/constants/dashboard'
import { getQueryClient } from '@shared/utils/get-query-client'
import { playerRecentQuerySchema } from '@server/dto/fe'
import { readDashboardOverview } from '@server/service/dashboard-reads'
import { parseQueryParams, readPlayerPlayDays, readPlayerProfile, readPlayerRecent, readPlayerStats } from '@server/service/public-reads'

const RECENT_PARAMS = { page: 1, limit: DASHBOARD_RECENT_PLAYS_LIMIT }

type MemberDashboardContentProps = {
    user: SessionUser
}

/** Reads what the first screen of the signed-in home shows (profile, overview, lamps, daily play time, recent plays) on the server. */
export const MemberDashboardContent = async ({ user }: MemberDashboardContentProps) => {
    const loginId = user.login_id
    const queryClient = getQueryClient()
    await Promise.all([
        queryClient.prefetchQuery({ ...playerQueryOptions(loginId), queryFn: () => readPlayerProfile(loginId) }),
        queryClient.prefetchQuery({
            ...dashboardOverviewQueryOptions(),
            queryFn: () => readDashboardOverview({ id: user.id, loginId }),
        }),
        queryClient.prefetchQuery({ ...playerStatsQueryOptions(loginId), queryFn: () => readPlayerStats(loginId) }),
        queryClient.prefetchQuery({ ...playerPlayDaysQueryOptions(loginId), queryFn: () => readPlayerPlayDays(loginId) }),
        queryClient.prefetchQuery({
            ...playerRecentQueryOptions(loginId, RECENT_PARAMS),
            queryFn: () => readPlayerRecent(loginId, parseQueryParams(playerRecentQuerySchema, RECENT_PARAMS)),
        }),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <MemberDashboardGrid userId={user.id} loginId={loginId} />
        </HydrationBoundary>
    )
}
