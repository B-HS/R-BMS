import { queryOptions, useQuery } from '@tanstack/react-query'
import { useSessionUser } from '@entities/auth.query'
import type {
    DashboardCalendar,
    DashboardCourses,
    DashboardOverview,
    DashboardReplays,
    DashboardRivals,
    DashboardTableProgress,
} from '@entities/dashboard.type'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope } from '@shared/lib/fetch'

const DASHBOARD_PATH = '/fe/me/dashboard'

export const dashboardOverviewQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.DASHBOARD.OVERVIEW,
        queryFn: () => fetchEnvelope<DashboardOverview>(buildApiUrl(`${DASHBOARD_PATH}/overview`)),
    })

export const dashboardCalendarQueryOptions = (days: number) =>
    queryOptions({
        queryKey: QUERY_KEY.DASHBOARD.CALENDAR(days),
        queryFn: () => fetchEnvelope<DashboardCalendar>(buildApiUrl(`${DASHBOARD_PATH}/calendar`, { days })),
    })

export const dashboardTableProgressQueryOptions = (tableId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.DASHBOARD.TABLE_PROGRESS(tableId),
        queryFn: () => fetchEnvelope<DashboardTableProgress>(buildApiUrl(`${DASHBOARD_PATH}/tables/${encodeURIComponent(tableId)}/progress`)),
    })

export const dashboardCoursesQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.DASHBOARD.COURSES,
        queryFn: () => fetchEnvelope<DashboardCourses>(buildApiUrl(`${DASHBOARD_PATH}/courses`)),
    })

export const dashboardRivalsQueryOptions = (limit: number) =>
    queryOptions({
        queryKey: QUERY_KEY.DASHBOARD.RIVALS(limit),
        queryFn: () => fetchEnvelope<DashboardRivals>(buildApiUrl(`${DASHBOARD_PATH}/rivals`, { limit })),
    })

export const dashboardReplaysQueryOptions = (limit: number) =>
    queryOptions({
        queryKey: QUERY_KEY.DASHBOARD.REPLAYS(limit),
        queryFn: () => fetchEnvelope<DashboardReplays>(buildApiUrl(`${DASHBOARD_PATH}/replays`, { limit })),
    })

export const useDashboardOverview = () => useQuery(dashboardOverviewQueryOptions())

export const useDashboardCalendar = (days: number) => useQuery(dashboardCalendarQueryOptions(days))

export const useDashboardTableProgress = (tableId: string) => useQuery(dashboardTableProgressQueryOptions(tableId))

export const useDashboardCourses = () => useQuery(dashboardCoursesQueryOptions())

export const useDashboardRivals = (limit: number, enabled = true) => useQuery({ ...dashboardRivalsQueryOptions(limit), enabled })

export const useDashboardReplays = (limit: number) => useQuery(dashboardReplaysQueryOptions(limit))

/**
 * Whether the signed-in browser session is still the player the server drew the dashboard for. It stays true while the session is being
 * read (the server's own answer stands) and turns false once the session ends or changes, for instance after a sign-out in another tab.
 */
export const useAlignedViewer = (initialUserId: string) => {
    const { user, isPending } = useSessionUser()
    return isPending || user?.id === initialUserId
}
