import type { FC } from 'react'
import { Skeleton } from '@shared/ui/skeleton'

type RowListSkeletonProps = {
    rows?: number
    label?: string
}

export const RowListSkeleton: FC<RowListSkeletonProps> = ({ rows = 3, label = '불러오는 중' }) => (
    <ul role='status' aria-label={label} aria-busy='true' className='bg-card flex flex-col'>
        {Array.from({ length: rows }, (_, index) => (
            <li key={index} className='border-border flex items-center gap-3 border-b px-3 py-2'>
                <Skeleton className='size-8 shrink-0' />
                <div className='flex min-w-0 flex-1 flex-col gap-1.5'>
                    <Skeleton className='h-3.5 w-40 max-w-full' />
                    <Skeleton className='h-3 w-64 max-w-full' />
                </div>
            </li>
        ))}
    </ul>
)
