import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Skeleton } from '@shared/ui/skeleton'

type TableSkeletonProps = {
    rows?: number
    label?: string
}

export const TableSkeleton: FC<TableSkeletonProps> = ({ rows = 6, label }) => {
    const t = useTranslations('pages.shared')

    return (
        <div role='status' aria-label={label ?? t('loading')} aria-busy='true' className='flex flex-col gap-2'>
            {Array.from({ length: rows }, (_, index) => (
                <Skeleton key={index} className='h-6 w-full' />
            ))}
        </div>
    )
}
