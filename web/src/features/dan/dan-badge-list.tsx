import type { FC } from 'react'
import { DanBadge } from '@features/dan/dan-badge'
import type { DisplayedDan } from '@entities/dan.type'

export const DanBadgeList: FC<{ dans: readonly DisplayedDan[] }> = ({ dans }) => {
    if (dans.length === 0) return null

    return (
        <ul aria-label='표시 단위' className='flex flex-wrap items-center gap-1'>
            {dans.map((dan) => (
                <li key={dan.system}>
                    <DanBadge dan={dan} />
                </li>
            ))}
        </ul>
    )
}
