import { z } from 'zod'
import { assistGradeSchema, clearLampSchema } from '@server/dto/common'

export const DASHBOARD_CALENDAR_DAYS_DEFAULT = 182
export const DASHBOARD_CALENDAR_DAYS_MAX = 371
export const DASHBOARD_STREAK_WINDOW_DAYS = 371
export const DASHBOARD_RIVALS_LIMIT_DEFAULT = 10
export const DASHBOARD_RIVALS_LIMIT_MAX = 20
export const DASHBOARD_REPLAYS_LIMIT_DEFAULT = 5
export const DASHBOARD_REPLAYS_LIMIT_MAX = 20

/** The nine groups the 27 steps of a DJ grade fall into, lowest first. A step belongs to the group its label starts with (`AA+` is AA, `MAX-` is MAX). */
export const DASHBOARD_GRADE_GROUPS = ['F', 'E', 'D', 'C', 'B', 'A', 'AA', 'AAA', 'MAX'] as const

export type DashboardGradeGroup = (typeof DASHBOARD_GRADE_GROUPS)[number]

export const dashboardCalendarQuerySchema = z.object({
    days: z.coerce.number().int().min(1).max(DASHBOARD_CALENDAR_DAYS_MAX).default(DASHBOARD_CALENDAR_DAYS_DEFAULT),
})

export const dashboardRivalsQuerySchema = z.object({
    limit: z.coerce.number().int().min(1).max(DASHBOARD_RIVALS_LIMIT_MAX).default(DASHBOARD_RIVALS_LIMIT_DEFAULT),
})

export const dashboardReplaysQuerySchema = z.object({
    limit: z.coerce.number().int().min(1).max(DASHBOARD_REPLAYS_LIMIT_MAX).default(DASHBOARD_REPLAYS_LIMIT_DEFAULT),
})

export type DashboardCalendarQuery = z.infer<typeof dashboardCalendarQuerySchema>
export type DashboardRivalsQuery = z.infer<typeof dashboardRivalsQuerySchema>
export type DashboardReplaysQuery = z.infer<typeof dashboardReplaysQuerySchema>

const countSchema = z.number().int().min(0)

export const dashboardTilesOutputSchema = z.object({
    total_plays: countSchema,
    played_charts: countSchema,
    cleared_charts: countSchema,
    hard_or_better: countSchema,
    full_combo_or_better: countSchema,
    play_ms_30d: countSchema,
    active_days_30d: countSchema,
    streak_days: countSchema,
    best_streak_days: countSchema,
    this_week_plays: countSchema,
    last_played_at: z.number().int().nullable(),
})

export const dashboardGradeOutputSchema = z.object({ grade: z.enum(DASHBOARD_GRADE_GROUPS), count: countSchema })

export const dashboardRankOutputSchema = z.object({ position: z.number().int().min(1).nullable(), total_players: countSchema })

export const dashboardOverviewOutputSchema = z.object({
    tiles: dashboardTilesOutputSchema,
    grades: z.array(dashboardGradeOutputSchema),
    rank: dashboardRankOutputSchema,
})

export const dashboardPlayDayOutputSchema = z.object({ date: z.string(), plays: countSchema, play_ms: countSchema })

export const dashboardCalendarOutputSchema = z.object({
    days: z.array(dashboardPlayDayOutputSchema),
    total_plays: countSchema,
    active_days: countSchema,
    streak_days: countSchema,
    best_streak_days: countSchema,
})

export const dashboardLampCountOutputSchema = z.object({ clear: clearLampSchema, assist_grade: assistGradeSchema, count: countSchema })

export const dashboardLevelProgressOutputSchema = z.object({
    level: z.string(),
    total: countSchema,
    played: countSchema,
    cleared: countSchema,
    hard_or_better: countSchema,
    lamps: z.array(dashboardLampCountOutputSchema),
})

export const dashboardTableProgressOutputSchema = z.object({
    table_id: z.string(),
    name: z.string(),
    symbol: z.string(),
    total: countSchema,
    played: countSchema,
    lamps: z.array(dashboardLampCountOutputSchema),
    levels: z.array(dashboardLevelProgressOutputSchema),
})

export const dashboardCoursesOutputSchema = z.object({
    groups: z.array(z.object({ id: z.string(), group: z.string(), total: countSchema, played: countSchema, cleared: countSchema })),
    recent: z.array(
        z.object({ course_hash: z.string(), name: z.string(), clear: clearLampSchema, ex_score: countSchema, played_at: z.number().int() }),
    ),
})

export const dashboardActivityOutputSchema = z.object({
    score_id: z.string(),
    player: z.object({ id: z.string() }),
    player_name: z.string(),
    clear: clearLampSchema,
    assist_grade: assistGradeSchema,
    ex_score: countSchema,
    minbp: z.number().int(),
    max_combo: countSchema,
    played_at: z.number().int(),
    scratch_auto: z.boolean(),
    chart: z.object({ md5: z.string(), sha256: z.string(), title: z.string(), artist: z.string(), level: z.number().int().nullable() }),
})

export const dashboardRivalsOutputSchema = z.object({
    rivals: z.array(z.object({ login_id: z.string(), name: z.string() })),
    recent: z.array(dashboardActivityOutputSchema),
    versus: z.array(z.object({ login_id: z.string(), common: countSchema, wins: countSchema, losses: countSchema, draws: countSchema })),
})

export const dashboardReplaysOutputSchema = z.object({
    replays: z.array(
        z.object({
            id: z.string(),
            created_at: z.number().int(),
            chart: z.object({ md5: z.string(), sha256: z.string(), title: z.string(), level: z.number().int().nullable() }),
            clear: clearLampSchema.nullable(),
            ex_score: countSchema.nullable(),
        }),
    ),
})

export type DashboardTilesOutput = z.infer<typeof dashboardTilesOutputSchema>
export type DashboardGradeOutput = z.infer<typeof dashboardGradeOutputSchema>
export type DashboardRankOutput = z.infer<typeof dashboardRankOutputSchema>
export type DashboardOverviewOutput = z.infer<typeof dashboardOverviewOutputSchema>
export type DashboardPlayDayOutput = z.infer<typeof dashboardPlayDayOutputSchema>
export type DashboardCalendarOutput = z.infer<typeof dashboardCalendarOutputSchema>
export type DashboardLampCountOutput = z.infer<typeof dashboardLampCountOutputSchema>
export type DashboardLevelProgressOutput = z.infer<typeof dashboardLevelProgressOutputSchema>
export type DashboardTableProgressOutput = z.infer<typeof dashboardTableProgressOutputSchema>
export type DashboardCoursesOutput = z.infer<typeof dashboardCoursesOutputSchema>
export type DashboardActivityOutput = z.infer<typeof dashboardActivityOutputSchema>
export type DashboardRivalsOutput = z.infer<typeof dashboardRivalsOutputSchema>
export type DashboardReplaysOutput = z.infer<typeof dashboardReplaysOutputSchema>
