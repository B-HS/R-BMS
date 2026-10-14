import type { FC } from 'react'
import { cn } from '@shared/lib/cn'
import { Skeleton } from '@shared/ui/skeleton'

type DashboardPanelLoadingProps = {
    label: string
    className?: string
}

export const DashboardPanelLoading: FC<DashboardPanelLoadingProps> = ({ label, className }) => (
    <div role='status' aria-label={label} aria-busy='true' className={cn('flex min-h-24 min-w-0 flex-1 flex-col', className)}>
        <Skeleton className='min-h-0 flex-1' />
    </div>
)
