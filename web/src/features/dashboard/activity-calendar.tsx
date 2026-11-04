import type { FC } from 'react'
import { DASHBOARD_CALENDAR_LEVEL_CLASS_NAMES, DASHBOARD_CALENDAR_LEVELS } from '@shared/constants/dashboard'
import { cn } from '@shared/lib/cn'

type ActivityCalendarDay = {
    date: string
    plays: number
}

type ActivityCalendarProps = {
    days: readonly ActivityCalendarDay[]
    label: string
    cellLabel: (day: ActivityCalendarDay) => string
    legend: { less: string; more: string }
}

const DAYS_PER_WEEK = 7
const DAYS_FROM_MONDAY_TO_SUNDAY = 6
const FIRST_GRID_ROW = 1

const calendarLevelOf = (plays: number) => DASHBOARD_CALENDAR_LEVELS.findLastIndex((threshold) => plays >= threshold)

const mondayBasedWeekdayOf = (date: string) => (new Date(`${date}T00:00:00Z`).getUTCDay() + DAYS_FROM_MONDAY_TO_SUNDAY) % DAYS_PER_WEEK

/**
 * The days as weeks side by side, Monday at the top. Each day is a square shaded by how many plays it holds, and the first day starts in the
 * row of its weekday so the rows stay aligned. The whole grid is one image for assistive technology and each square carries its own hover title.
 */
export const ActivityCalendar: FC<ActivityCalendarProps> = ({ days, label, cellLabel, legend }) => (
    <div className='flex min-w-0 flex-col gap-1.5'>
        <div role='img' aria-label={label} className='grid auto-cols-fr grid-flow-col grid-rows-[repeat(7,auto)] gap-px'>
            {days.map((day, index) => (
                <span
                    key={day.date}
                    title={cellLabel(day)}
                    className={cn('aspect-square min-w-0', DASHBOARD_CALENDAR_LEVEL_CLASS_NAMES[calendarLevelOf(day.plays)])}
                    style={index === 0 ? { gridRowStart: mondayBasedWeekdayOf(day.date) + FIRST_GRID_ROW } : undefined}
                />
            ))}
        </div>
        <div className='text-muted-foreground text-2xs flex items-center justify-between gap-2 tabular-nums'>
            <span>
                {days[0]?.date} – {days.at(-1)?.date}
            </span>
            <span className='flex items-center gap-1'>
                {legend.less}
                {DASHBOARD_CALENDAR_LEVEL_CLASS_NAMES.map((className) => (
                    <span key={className} aria-hidden='true' className={cn('size-2.5', className)} />
                ))}
                {legend.more}
            </span>
        </div>
    </div>
)
