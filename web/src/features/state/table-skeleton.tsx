import type { FC } from 'react'
import { Skeleton } from '@shared/ui/skeleton'

type TableSkeletonProps = {
    rows?: number
    label?: string
}

export const TableSkeleton: FC<TableSkeletonProps> = ({ rows = 6, label = '불러오는 중' }) => (
    <div role='status' aria-label={label} aria-busy='true' className='flex flex-col gap-2'>
        {Array.from({ length: rows }, (_, index) => (
            <Skeleton key={index} className='h-6 w-full' />
        ))}
    </div>
)
