const MINUTE_MS = 60_000
const HOUR_MS = 3_600_000
const DAY_MS = 86_400_000
const ISO_DATE_LENGTH = 10
const KST_UTC_OFFSET_HOURS = 9

export const PLAY_DAYS_WINDOW = 30
export const PLAY_SESSION_GAP_MS = 10 * MINUTE_MS
export const PLAY_DEFAULT_MS = 2 * MINUTE_MS

/**
 * Asia/Seoul is UTC+9 all year (no daylight saving since 1988), so a day boundary there is plain arithmetic on the epoch and needs no per-play
 * `Intl` call. This must stay equal to the offset of `DISPLAY_TIME_ZONE` in `@shared/lib/format`; a test compares the two through `formatDateTime`.
 */
export const KST_UTC_OFFSET_MS = KST_UTC_OFFSET_HOURS * HOUR_MS

export type PlayDay = { date: string; plays: number; play_ms: number }

/** The epoch millisecond of 00:00 KST on the day that holds `at`. */
export const kstDayStartOf = (at: number) => Math.floor((at + KST_UTC_OFFSET_MS) / DAY_MS) * DAY_MS - KST_UTC_OFFSET_MS

const toKstDate = (dayStart: number) => new Date(dayStart + KST_UTC_OFFSET_MS).toISOString().slice(0, ISO_DATE_LENGTH)

/** The half-open range `[from, to)` of the last `days` KST days (`PLAY_DAYS_WINDOW` by default), the last one being the day that starts at `todayStart`. */
export const playDaysWindowOf = (todayStart: number, days = PLAY_DAYS_WINDOW) => ({ from: todayStart - (days - 1) * DAY_MS, to: todayStart + DAY_MS })

/**
 * Estimates how long a player played on each of the last `days` KST days (`PLAY_DAYS_WINDOW` by default) from the instants their plays ended.
 *
 * The server stores no play length, so each play is given the gap to the play before it when that gap is at most `PLAY_SESSION_GAP_MS`, and
 * `PLAY_DEFAULT_MS` otherwise (a first play, or one after a break). A play counts for the day it ended on. `previousPlayedAt` is the latest play
 * before the window: it only joins the first play of the window to a session that began the day before. The stages of a course are score rows of
 * their own, so course rows are not read and nothing is counted twice.
 */
export const summarisePlayDays = (params: { playedAts: readonly number[]; previousPlayedAt: number | null; todayStart: number; days?: number }) => {
    const windowDays = params.days ?? PLAY_DAYS_WINDOW
    const { from, to } = playDaysWindowOf(params.todayStart, windowDays)
    const ordered = params.playedAts.filter((playedAt) => playedAt >= from && playedAt < to).toSorted((left, right) => left - right)
    const plays = ordered.map((playedAt, index) => {
        const previousPlayedAt = index === 0 ? params.previousPlayedAt : ordered[index - 1]
        const gap = previousPlayedAt === null ? Number.POSITIVE_INFINITY : playedAt - previousPlayedAt
        return { dayIndex: Math.floor((playedAt - from) / DAY_MS), playMs: gap <= PLAY_SESSION_GAP_MS ? gap : PLAY_DEFAULT_MS }
    })
    const playsOfDay = Map.groupBy(plays, (play) => play.dayIndex)
    const days = Array.from({ length: windowDays }, (_, dayIndex): PlayDay => {
        const ofDay = playsOfDay.get(dayIndex) ?? []
        return { date: toKstDate(from + dayIndex * DAY_MS), plays: ofDay.length, play_ms: ofDay.reduce((sum, play) => sum + play.playMs, 0) }
    })
    return { days, total_plays: plays.length, total_ms: days.reduce((sum, day) => sum + day.play_ms, 0) }
}
