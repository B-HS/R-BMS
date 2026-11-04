const DAYS_PER_WEEK = 7
const ISO_DATE_TIME_SUFFIX = 'T00:00:00Z'

type DayPlays = { plays: number }

/**
 * The run of consecutive days with at least one play that ends today, or yesterday when today has no play yet (a day that is not over
 * must not zero a streak). `days` is oldest first and its last element is today.
 */
export const currentStreakOf = (days: readonly DayPlays[]) => {
    const endIndex = days.at(-1)?.plays ? days.length - 1 : days.length - 2
    const breakIndex = days.slice(0, endIndex + 1).findLastIndex((day) => day.plays === 0)
    return Math.max(endIndex - breakIndex, 0)
}

/** The longest run of consecutive days with at least one play. */
export const bestStreakOf = (days: readonly DayPlays[]) =>
    days.reduce(
        (state, day) => {
            const run = day.plays > 0 ? state.run + 1 : 0
            return { run, best: Math.max(state.best, run) }
        },
        { run: 0, best: 0 },
    ).best

/** The days from this week's Monday through today. `days` is oldest first, its last element is today and carries a `YYYY-MM-DD` KST date. */
export const thisWeekOf = <T extends DayPlays & { date: string }>(days: readonly T[]) => {
    const today = days.at(-1)
    if (!today) return []
    const weekday = new Date(`${today.date}${ISO_DATE_TIME_SUFFIX}`).getUTCDay()
    const daysSinceMonday = (weekday + DAYS_PER_WEEK - 1) % DAYS_PER_WEEK
    return days.slice(-(daysSinceMonday + 1))
}
