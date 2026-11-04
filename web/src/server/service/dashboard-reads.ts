import 'server-only'
import { compose } from '@server/compose'
import type { DashboardCalendarQuery, DashboardReplaysQuery, DashboardRivalsQuery } from '@server/dto/dashboard'
import type { AuthUser } from '@server/lib/auth-context'
import { toDashboardRank } from '@server/service/domain/dashboard/dashboard.service'
import { kstDayStartOf } from '@server/service/domain/fe/player-play-days'
import {
    getDashboardCalendarCached,
    getDashboardCoursesCached,
    getDashboardOverviewCached,
    getDashboardReplaysCached,
    getDashboardTableProgressCached,
    getLeaderboardPositionsCached,
} from '@server/service/read-cache'

/** The session user as the dashboard reads need them: the id keys the queries, the login id names the cache tag. */
export type DashboardViewer = Pick<AuthUser, 'id' | 'loginId'>

/**
 * The KST day the request lands on is part of the cache key of the overview and the calendar, so a cached window moves on at midnight
 * (and a cached week at Monday) instead of at the cache lifetime. The rank comes from the player ranking's own cache, which a
 * score submit drops with the dashboard's.
 */
export const readDashboardOverview = async (viewer: DashboardViewer) => {
    const [overview, standings] = await Promise.all([
        getDashboardOverviewCached({ userId: viewer.id, loginId: viewer.loginId, todayStart: kstDayStartOf(Date.now()) }),
        getLeaderboardPositionsCached(),
    ])
    return { ...overview, rank: toDashboardRank(standings, viewer.id) }
}

export const readDashboardCalendar = async (viewer: DashboardViewer, query: DashboardCalendarQuery) =>
    getDashboardCalendarCached({ userId: viewer.id, loginId: viewer.loginId, todayStart: kstDayStartOf(Date.now()), days: query.days })

export const readDashboardTableProgress = async (viewer: DashboardViewer, tableId: string) =>
    getDashboardTableProgressCached({ userId: viewer.id, loginId: viewer.loginId, tableId })

export const readDashboardCourses = async (viewer: DashboardViewer) => getDashboardCoursesCached({ userId: viewer.id, loginId: viewer.loginId })

/** A rival's plays arrive without the viewer's doing, so no score submit of the viewer could drop a cached copy: this read is never cached. */
export const readDashboardRivals = async (viewer: DashboardViewer, query: DashboardRivalsQuery) =>
    compose().dashboardService.rivals({ userId: viewer.id, limit: query.limit })

export const readDashboardReplays = async (viewer: DashboardViewer, query: DashboardReplaysQuery) =>
    getDashboardReplaysCached({ userId: viewer.id, loginId: viewer.loginId, limit: query.limit })
