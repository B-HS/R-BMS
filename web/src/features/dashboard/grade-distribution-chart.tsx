import type { FC } from 'react'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'

type GradeDistributionChartGrade = {
    grade: string
    count: number
}

type GradeDistributionChartProps = {
    grades: readonly GradeDistributionChartGrade[]
    label: string
}

const MIN_BAR_PERCENT = 2

/**
 * One vertical bar per DJ grade group, tall in proportion to the most frequent group. A group nobody holds is a bar of no height,
 * so the groups keep their places. The bars do not animate.
 */
export const GradeDistributionChart: FC<GradeDistributionChartProps> = ({ grades, label }) => {
    const peak = Math.max(1, ...grades.map((entry) => entry.count))

    return (
        <ul aria-label={label} className='grid min-w-0 auto-cols-fr grid-flow-col gap-1'>
            {grades.map((entry) => (
                <li key={entry.grade} className='flex min-w-0 flex-col items-stretch gap-0.5'>
                    <div className='border-border flex h-20 items-end border-b'>
                        <div
                            role='img'
                            aria-label={`${entry.grade} ${formatCount(entry.count)}`}
                            className='bg-muted-foreground/70 w-full'
                            style={{ height: entry.count > 0 ? `${Math.max(MIN_BAR_PERCENT, (entry.count / peak) * PERCENT_SCALE)}%` : 0 }}
                        />
                    </div>
                    <span className='text-center text-xs font-medium'>{entry.grade}</span>
                    <span className='text-muted-foreground text-2xs text-center tabular-nums'>{formatCount(entry.count)}</span>
                </li>
            ))}
        </ul>
    )
}
