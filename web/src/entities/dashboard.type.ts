import type {
    DashboardCalendarOutput,
    DashboardCoursesOutput,
    DashboardGradeOutput,
    DashboardOverviewOutput,
    DashboardRankOutput,
    DashboardReplaysOutput,
    DashboardRivalsOutput,
    DashboardTableProgressOutput,
    DashboardTilesOutput,
} from '@server/dto/dashboard'

export {
    DASHBOARD_CALENDAR_DAYS_DEFAULT as DASHBOARD_CALENDAR_DAYS,
    DASHBOARD_REPLAYS_LIMIT_DEFAULT as DASHBOARD_REPLAYS_LIMIT,
    DASHBOARD_RIVALS_LIMIT_DEFAULT as DASHBOARD_RIVALS_LIMIT,
} from '@server/dto/dashboard'

export type DashboardOverview = DashboardOverviewOutput
export type DashboardTiles = DashboardTilesOutput
export type DashboardGrade = DashboardGradeOutput
export type DashboardRank = DashboardRankOutput
export type DashboardCalendar = DashboardCalendarOutput
export type DashboardCalendarDay = DashboardCalendar['days'][number]
export type DashboardTableProgress = DashboardTableProgressOutput
export type DashboardLevelProgress = DashboardTableProgress['levels'][number]
export type DashboardCourses = DashboardCoursesOutput
export type DashboardRivals = DashboardRivalsOutput
export type DashboardReplays = DashboardReplaysOutput
export type DashboardReplay = DashboardReplays['replays'][number]
