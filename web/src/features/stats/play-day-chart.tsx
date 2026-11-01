import type { FC } from 'react'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { cn } from '@shared/lib/cn'

type PlayDayChartDay = { date: string; playMs: number; label: string }

type PlayDayChartProps = {
    days: readonly PlayDayChartDay[]
    listLabel: string
}

const MIN_BAR_PERCENT = 2
const CHART_HEIGHT_CLASS = 'h-28'

/**
 * One bar per day, tall in proportion to the busiest day. A day without play is a bar of no height, so every day of the range keeps its column.
 * Each bar is described by `label` for assistive technology and for a hover title.
 */
export const PlayDayChart: FC<PlayDayChartProps> = ({ days, listLabel }) => {
    const peakMs = Math.max(1, ...days.map((day) => day.playMs))

    return (
        <div className='flex min-w-0 flex-col gap-1'>
            <ul aria-label={listLabel} className={cn('border-border flex items-end gap-px border-b', CHART_HEIGHT_CLASS)}>
                {days.map((day) => (
                    <li key={day.date} className='flex h-full min-w-0 flex-1 items-end'>
                        <div
                            role='img'
                            aria-label={day.label}
                            title={day.label}
                            className='bg-primary w-full'
                            style={{ height: day.playMs > 0 ? `${Math.max(MIN_BAR_PERCENT, (day.playMs / peakMs) * PERCENT_SCALE)}%` : 0 }}
                        />
                    </li>
                ))}
            </ul>
            <div className='text-muted-foreground text-2xs flex justify-between tabular-nums'>
                <span>{days[0]?.date}</span>
                <span>{days.at(-1)?.date}</span>
            </div>
        </div>
    )
}
