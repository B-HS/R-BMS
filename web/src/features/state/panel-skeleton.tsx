import type { FC } from 'react'
import { Skeleton } from '@shared/ui/skeleton'

const HEIGHT_CLASS = { panel: 'h-24', table: 'h-64', page: 'h-96' }

type PanelSkeletonProps = {
    height?: keyof typeof HEIGHT_CLASS
    label?: string
}

export const PanelSkeleton: FC<PanelSkeletonProps> = ({ height = 'panel', label = '불러오는 중' }) => (
    <div role='status' aria-label={label} aria-busy='true' className='bg-card p-3'>
        <Skeleton className={HEIGHT_CLASS[height]} />
    </div>
)
