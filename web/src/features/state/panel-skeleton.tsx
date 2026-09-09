import type { FC } from 'react'
import { Skeleton } from '@shared/ui/skeleton'

export const PanelSkeleton: FC<{ height?: 'panel' | 'table' | 'page' }> = ({ height = 'panel' }) => {
    const heightClass = height === 'page' ? 'h-96' : height === 'table' ? 'h-64' : 'h-24'
    return (
        <div className='bg-card p-3' aria-hidden>
            <Skeleton className={heightClass} />
        </div>
    )
}
