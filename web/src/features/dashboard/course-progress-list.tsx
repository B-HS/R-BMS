import type { FC } from 'react'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'

type CourseProgressListGroup = {
    id: string
    group: string
    total: number
    played: number
    cleared: number
}

type CourseProgressListProps = {
    groups: readonly CourseProgressListGroup[]
    label: string
    describeGroup: (group: CourseProgressListGroup) => string
}

const percentOf = (part: number, whole: number) => (whole === 0 ? 0 : (part / whole) * PERCENT_SCALE)

/** One row per course group: a bar of the courses played with the cleared ones filled darker, and `cleared/total` beside it. */
export const CourseProgressList: FC<CourseProgressListProps> = ({ groups, label, describeGroup }) => (
    <ul aria-label={label} className='flex flex-col gap-1.5'>
        {groups.map((group) => (
            <li key={group.id} className='grid grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_4rem] items-center gap-2 text-xs'>
                <span className='truncate'>{group.group}</span>
                <span role='img' aria-label={describeGroup(group)} className='bg-muted relative h-2 min-w-0'>
                    <span
                        className='bg-muted-foreground/50 absolute inset-y-0 left-0'
                        style={{ width: `${percentOf(group.played, group.total)}%` }}
                    />
                    <span className='bg-primary absolute inset-y-0 left-0' style={{ width: `${percentOf(group.cleared, group.total)}%` }} />
                </span>
                <span className='text-right tabular-nums'>
                    {formatCount(group.cleared)}/{formatCount(group.total)}
                </span>
            </li>
        ))}
    </ul>
)
