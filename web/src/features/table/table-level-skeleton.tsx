import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Skeleton } from '@shared/ui/skeleton'

type TableLevelSkeletonProps = {
    levels: { label: string; count: number }[]
    previewCards: number
}

export const TableLevelSkeleton: FC<TableLevelSkeletonProps> = ({ levels, previewCards }) => {
    const t = useTranslations('pages.table')
    const tShared = useTranslations('pages.shared')

    return (
        <div role='status' aria-busy='true' className='flex min-w-0 flex-col gap-px'>
            <span className='sr-only'>{t('loading')}</span>
            <div className='bg-background flex h-12 items-center gap-2 border-b px-3'>
                <Skeleton className='h-7 w-56' />
                <Skeleton className='hidden h-7 w-48 md:block' />
            </div>
            {levels.map((level, index) => (
                <div key={level.label} className='flex min-w-0 flex-col gap-px'>
                    <div className='bg-muted flex h-12 items-center gap-3 px-3'>
                        <span className='w-12 text-sm font-semibold tabular-nums'>{level.label}</span>
                        <span className='text-muted-foreground ml-auto text-xs tabular-nums'>{tShared('songCount', { count: level.count })}</span>
                    </div>
                    {index === 0 && (
                        <div className='bg-border grid grid-cols-2 gap-px sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6'>
                            {Array.from({ length: Math.min(level.count, previewCards) }, (_, cardIndex) => (
                                <div key={cardIndex} className='bg-card flex h-14 flex-col justify-between py-1.5 pr-1.5 pl-3'>
                                    <Skeleton className='h-3 w-4/5' />
                                    <Skeleton className='h-3 w-1/2' />
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            ))}
        </div>
    )
}
