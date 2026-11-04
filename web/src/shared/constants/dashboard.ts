export const DASHBOARD_GRID_CLASS_NAME =
    'grid min-w-0 grid-cols-1 gap-px bg-border @min-[45rem]/dashboard:grid-cols-2 @min-[62rem]/dashboard:grid-cols-3 @min-[96rem]/dashboard:grid-cols-4'
export const DASHBOARD_WIDE_PANEL_CLASS_NAME = '@min-[62rem]/dashboard:col-span-2'
export const DASHBOARD_FILL_PANEL_CLASS_NAME = '@min-[96rem]/dashboard:col-span-2'
export const DASHBOARD_FULL_PANEL_CLASS_NAME = 'col-span-full'

export const DASHBOARD_MEMBER_GRID_CLASS_NAME = `col-span-full grid-flow-dense ${DASHBOARD_GRID_CLASS_NAME}`

export const DASHBOARD_RECENT_PLAYS_LIMIT = 20
export const DASHBOARD_COMMUNITY_ACTIVITY_LIMIT = 10
export const DASHBOARD_FOLDERS_SHOWN = 5
export const DASHBOARD_GOALS_SHOWN = 5
export const DASHBOARD_CHALLENGES_SHOWN = 3
export const DASHBOARD_RECENT_COURSES_SHOWN = 5
export const DASHBOARD_IN_VIEW_ROOT_MARGIN = '240px'

/** The play counts a calendar day needs to reach each shade, lightest first; the shade of a day is the last one its plays reach. */
export const DASHBOARD_CALENDAR_LEVELS = [0, 1, 3, 6, 10] as const

export const DASHBOARD_CALENDAR_LEVEL_CLASS_NAMES = ['bg-muted', 'bg-primary/25', 'bg-primary/45', 'bg-primary/70', 'bg-primary'] as const

export const DASHBOARD_PROGRESS_TABLES = [
    { id: 'insane1', labelKey: 'insane1' },
    { id: 'stella-st', labelKey: 'stellaSt' },
    { id: 'stella-sl', labelKey: 'stellaSl' },
] as const
