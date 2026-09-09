import type { FC } from 'react'
import { Skeleton } from '@shared/ui/skeleton'

export const TableSkeleton: FC<{ rows?: number }> = ({ rows = 6 }) => (
    <div className='flex flex-col gap-2' aria-hidden>
        {Array.from({ length: rows }, (_, index) => (
            <Skeleton key={index} className='h-6 w-full' />
        ))}
    </div>
)
