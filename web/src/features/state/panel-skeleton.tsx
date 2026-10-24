import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Skeleton } from '@shared/ui/skeleton'

const HEIGHT_CLASS = { panel: 'h-24', table: 'h-64', page: 'h-96' }

type PanelSkeletonProps = {
    height?: keyof typeof HEIGHT_CLASS
    label?: string
}

export const PanelSkeleton: FC<PanelSkeletonProps> = ({ height = 'panel', label }) => {
    const t = useTranslations('pages.shared')

    return (
        <div role='status' aria-label={label ?? t('loading')} aria-busy='true' className='bg-card p-3'>
            <Skeleton className={HEIGHT_CLASS[height]} />
        </div>
    )
}
