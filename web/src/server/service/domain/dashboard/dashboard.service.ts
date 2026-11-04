import { CLEAR_LAMPS, MIN_CLEAR_LAMP_ID, NO_ASSIST_GRADE, clearLampFromId, clearLampToId, type AssistGrade } from '@server/dto/common'
import type { CourseCatalogOutput } from '@server/dto/course-catalog'
import {
    DASHBOARD_GRADE_GROUPS,
    DASHBOARD_STREAK_WINDOW_DAYS,
    type DashboardCalendarOutput,
    type DashboardCoursesOutput,
    type DashboardGradeGroup,
    type DashboardLampCountOutput,
    type DashboardLevelProgressOutput,
    type DashboardOverviewOutput,
    type DashboardRankOutput,
    type DashboardReplaysOutput,
    type DashboardRivalsOutput,
    type DashboardTableProgressOutput,
} from '@server/dto/dashboard'
import type { TableEntryOutput, TableRecordOutput } from '@server/dto/table-snapshot'
import { fillBlankText, type TableTextLookup } from '@server/service/domain/chart/chart.service'
import { bestStreakOf, currentStreakOf, thisWeekOf } from '@server/service/domain/dashboard/play-streak'
import { toActivityItem, type ActivityRow } from '@server/service/domain/fe/fe.service'
import { PLAY_DAYS_WINDOW, playDaysWindowOf, summarisePlayDays } from '@server/service/domain/fe/player-play-days'
import type { TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { getDjRank } from '@shared/lib/dj-rank'

export const DASHBOARD_RECENT_COURSES_LIMIT = 5

const EX_PER_NOTE = 2
export const HARD_LAMP_ID = clearLampToId('Hard')
const GRADE_SUFFIX = /[+-]$/
const UNPLAYED_LAMP = { clear: CLEAR_LAMPS[0], assist_grade: NO_ASSIST_GRADE } satisfies Omit<DashboardLampCountOutput, 'count'>

export type DashboardChartBestCounts = { played: number; cleared: number; hardOrBetter: number; fullComboOrBetter: number }

/** How many bests share one EX score on a chart of `notes` notes; the grade follows from the pair. */
export type DashboardGradeInput = { exScore: number; notes: number; count: number }

export type DashboardScoreSummary = { total: number; lastPlayedAt: number | null }

export type DashboardRivalRow = { userId: string; loginId: string; name: string }

export type DashboardVersusRow = { rivalId: string; common: number; wins: number; losses: number }

export type DashboardCourseBestRow = { courseHash: string; clear: number }

export type DashboardCourseScoreRow = { courseHash: string; name: string; clear: number; exScore: number; playedAt: number }

export type DashboardReplayRow = {
    id: string
    createdAt: number
    chartSha256: string
    chartMd5: string | null
    title: string | null
    artist: string | null
    level: number | null
    clear: number | null
    exScore: number | null
}

/** Every player who has a best, with the rank the player ranking gives them (ties share a rank and the next one skips). */
export type DashboardStandings = { total_players: number; positions: Record<string, number> }

export type DashboardServiceDb = {
    countChartBests: (userId: string) => Promise<DashboardChartBestCounts>
    listGradeInputs: (userId: string) => Promise<DashboardGradeInput[]>
    summariseScores: (userId: string) => Promise<DashboardScoreSummary>
    listPlayedAts: (params: { userId: string; from: number; to: number }) => Promise<{ playedAts: number[]; previousPlayedAt: number | null }>
    listStandings: () => Promise<{ userId: string; position: number }[]>
    listCourseBests: (userId: string) => Promise<DashboardCourseBestRow[]>
    listRecentCourseScores: (params: { userId: string; limit: number }) => Promise<DashboardCourseScoreRow[]>
    listRivals: (userId: string) => Promise<DashboardRivalRow[]>
    listRecentRivalActivity: (params: { userIds: string[]; limit: number }) => Promise<ActivityRow[]>
    listVersus: (params: { userId: string; rivalIds: string[] }) => Promise<DashboardVersusRow[]>
    listRecentReplays: (params: { userId: string; limit: number }) => Promise<DashboardReplayRow[]>
}

export type DashboardServiceDeps = {
    db: DashboardServiceDb
    tableSnapshotService: Pick<TableSnapshotService, 'getDetail' | 'getRecords'>
    listCatalogGroups: () => Promise<CourseCatalogOutput>
    lookupTableText?: TableTextLookup
}

const gradeGroupOf = (label: string): DashboardGradeGroup | undefined =>
    DASHBOARD_GRADE_GROUPS.find((group) => group === label.replace(GRADE_SUFFIX, ''))

/**
 * Counts the bests of each grade group. A chart without a note count has no ceiling to grade against, so its best is left out.
 * Every group is listed, empty ones with a zero, so the chart that shows them keeps its nine bars.
 */
export const countGradeGroups = (inputs: readonly DashboardGradeInput[]) => {
    const byGroup = Map.groupBy(
        inputs.filter((input) => input.notes > 0),
        (input) => gradeGroupOf(getDjRank(input.exScore, EX_PER_NOTE * input.notes)),
    )
    return DASHBOARD_GRADE_GROUPS.map((grade) => ({ grade, count: (byGroup.get(grade) ?? []).reduce((sum, input) => sum + input.count, 0) }))
}

export const toDashboardStandings = (rows: readonly { userId: string; position: number }[]): DashboardStandings => ({
    total_players: rows.length,
    positions: Object.fromEntries(rows.map((row) => [row.userId, row.position])),
})

/** The player's place in `standings`: `null` when they have no best and so no place. */
export const toDashboardRank = (standings: DashboardStandings, userId: string): DashboardRankOutput => ({
    position: Object.hasOwn(standings.positions, userId) ? standings.positions[userId] : null,
    total_players: standings.total_players,
})

const assistGradeOf = (record: Pick<TableRecordOutput, 'clear' | 'assist_grade'>): AssistGrade =>
    record.clear === 'LightAssistEasy' ? record.assist_grade : NO_ASSIST_GRADE

const lampOf = (record: TableRecordOutput | undefined) => (record ? { clear: record.clear, assist_grade: assistGradeOf(record) } : UNPLAYED_LAMP)

/** Counts the lamps of `entries` from the best lamp down to NO PLAY, the order a stacked bar fills from the left. Lamps nobody holds are left out. */
const countLamps = (entries: readonly TableEntryOutput[], records: Readonly<Record<string, TableRecordOutput>>): DashboardLampCountOutput[] =>
    [
        ...Map.groupBy(
            entries.map((entry) => lampOf(records[entry.md5])),
            (lamp) => `${lamp.clear}:${lamp.assist_grade}`,
        ).values(),
    ]
        .map((group) => ({ ...group[0], count: group.length }))
        .toSorted((left, right) => clearLampToId(right.clear) - clearLampToId(left.clear) || right.assist_grade - left.assist_grade)

const levelProgressOf = (entries: readonly TableEntryOutput[], records: Readonly<Record<string, TableRecordOutput>>) => {
    const recorded = entries.flatMap((entry) => {
        const record = records[entry.md5]
        return record ? [clearLampToId(record.clear)] : []
    })
    return {
        total: entries.length,
        played: recorded.length,
        cleared: recorded.filter((lampId) => lampId >= MIN_CLEAR_LAMP_ID).length,
        hard_or_better: recorded.filter((lampId) => lampId >= HARD_LAMP_ID).length,
        lamps: countLamps(entries, records),
    }
}

export const createDashboardService = (deps: DashboardServiceDeps) => ({
    /** The tiles and grade bars. The player's rank is not part of it: it follows the player ranking's own cache (see `toDashboardRank`). */
    overview: async (params: { userId: string; todayStart: number }): Promise<Omit<DashboardOverviewOutput, 'rank'>> => {
        const { userId, todayStart } = params
        const [chartBests, gradeInputs, scores, history] = await Promise.all([
            deps.db.countChartBests(userId),
            deps.db.listGradeInputs(userId),
            deps.db.summariseScores(userId),
            deps.db.listPlayedAts({ userId, ...playDaysWindowOf(todayStart, DASHBOARD_STREAK_WINDOW_DAYS) }),
        ])
        const { days } = summarisePlayDays({
            playedAts: history.playedAts,
            previousPlayedAt: history.previousPlayedAt,
            todayStart,
            days: DASHBOARD_STREAK_WINDOW_DAYS,
        })
        const lastMonth = days.slice(-PLAY_DAYS_WINDOW)
        return {
            tiles: {
                total_plays: scores.total,
                played_charts: chartBests.played,
                cleared_charts: chartBests.cleared,
                hard_or_better: chartBests.hardOrBetter,
                full_combo_or_better: chartBests.fullComboOrBetter,
                play_ms_30d: lastMonth.reduce((sum, day) => sum + day.play_ms, 0),
                active_days_30d: lastMonth.filter((day) => day.plays > 0).length,
                streak_days: currentStreakOf(days),
                best_streak_days: bestStreakOf(days),
                this_week_plays: thisWeekOf(days).reduce((sum, day) => sum + day.plays, 0),
                last_played_at: scores.lastPlayedAt,
            },
            grades: countGradeGroups(gradeInputs),
        }
    },

    /** Every player's place in the player ranking, computed once and shared by all dashboards. */
    leaderboardStandings: async () => toDashboardStandings(await deps.db.listStandings()),

    calendar: async (params: { userId: string; todayStart: number; days: number }): Promise<DashboardCalendarOutput> => {
        const { userId, todayStart } = params
        const { playedAts, previousPlayedAt } = await deps.db.listPlayedAts({ userId, ...playDaysWindowOf(todayStart, params.days) })
        const summary = summarisePlayDays({ playedAts, previousPlayedAt, todayStart, days: params.days })
        return {
            days: summary.days,
            total_plays: summary.total_plays,
            active_days: summary.days.filter((day) => day.plays > 0).length,
            streak_days: currentStreakOf(summary.days),
            best_streak_days: bestStreakOf(summary.days),
        }
    },

    /** The player's lamps over one difficulty table, whole and per level; `null` when the server has no such table. */
    tableProgress: async (params: { userId: string; tableId: string }): Promise<DashboardTableProgressOutput | null> => {
        const [detail, snapshot] = await Promise.all([
            deps.tableSnapshotService.getDetail(params.tableId),
            deps.tableSnapshotService.getRecords(params),
        ])
        if (!detail || !snapshot) return null
        const levels = detail.levels.map(({ level, entries }): DashboardLevelProgressOutput => ({
            level,
            ...levelProgressOf(entries, snapshot.records),
        }))
        const { total, played, lamps } = levelProgressOf(
            detail.levels.flatMap((group) => group.entries),
            snapshot.records,
        )
        return { table_id: detail.table.id, name: detail.table.name, symbol: detail.table.symbol, total, played, lamps, levels }
    },

    courses: async (params: { userId: string }): Promise<DashboardCoursesOutput> => {
        const [catalog, bests, recent] = await Promise.all([
            deps.listCatalogGroups(),
            deps.db.listCourseBests(params.userId),
            deps.db.listRecentCourseScores({ userId: params.userId, limit: DASHBOARD_RECENT_COURSES_LIMIT }),
        ])
        const clearOf = new Map(bests.map((best) => [best.courseHash, best.clear]))
        return {
            groups: catalog.groups.flatMap((group) => {
                const active = group.courses.filter((course) => course.active)
                if (active.length === 0) return []
                return [
                    {
                        id: group.id,
                        group: group.name,
                        total: active.length,
                        played: active.filter((course) => clearOf.has(course.course_hash)).length,
                        cleared: active.filter((course) => (clearOf.get(course.course_hash) ?? 0) >= MIN_CLEAR_LAMP_ID).length,
                    },
                ]
            }),
            recent: recent.map((run) => ({
                course_hash: run.courseHash,
                name: run.name,
                clear: clearLampFromId(run.clear),
                ex_score: run.exScore,
                played_at: run.playedAt,
            })),
        }
    },

    rivals: async (params: { userId: string; limit: number }): Promise<DashboardRivalsOutput> => {
        const rivals = await deps.db.listRivals(params.userId)
        if (rivals.length === 0) return { rivals: [], recent: [], versus: [] }
        const rivalIds = rivals.map((rival) => rival.userId)
        const [recent, versus] = await Promise.all([
            deps.db.listRecentRivalActivity({ userIds: rivalIds, limit: params.limit }),
            deps.db.listVersus({ userId: params.userId, rivalIds }),
        ])
        const versusOf = new Map(versus.map((row) => [row.rivalId, row]))
        return {
            rivals: rivals.map((rival) => ({ login_id: rival.loginId, name: rival.name })),
            recent: recent.map((row) => toActivityItem(row, deps.lookupTableText)),
            versus: rivals.map((rival) => {
                const row = versusOf.get(rival.userId)
                const [common, wins, losses] = [row?.common ?? 0, row?.wins ?? 0, row?.losses ?? 0]
                return { login_id: rival.loginId, common, wins, losses, draws: common - wins - losses }
            }),
        }
    },

    replays: async (params: { userId: string; limit: number }): Promise<DashboardReplaysOutput> => {
        const rows = await deps.db.listRecentReplays(params)
        return {
            replays: rows.map((row) => {
                const text = fillBlankText({ md5: row.chartMd5 ?? '', title: row.title ?? '', artist: row.artist ?? '' }, deps.lookupTableText)
                return {
                    id: row.id,
                    created_at: row.createdAt,
                    chart: { md5: text.md5, sha256: row.chartSha256, title: text.title, level: row.level },
                    clear: row.clear === null ? null : clearLampFromId(row.clear),
                    ex_score: row.exScore,
                }
            }),
        }
    },
})

export type DashboardService = ReturnType<typeof createDashboardService>
