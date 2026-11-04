import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { cn } from '@shared/lib/cn'

const PLACEHOLDERS = [
    { id: 'lead', className: DASHBOARD_WIDE_PANEL_CLASS_NAME },
    { id: 'first', className: undefined },
    { id: 'second', className: undefined },
    { id: 'trail', className: DASHBOARD_WIDE_PANEL_CLASS_NAME },
]

/** Four blank panels that fit a signed-in and a signed-out home alike, shown until the server knows which of the two to draw. */
export const HomeBranchSkeleton: FC = () => {
    const t = useTranslations('pages.shared')

    return (
        <>
            {PLACEHOLDERS.map((placeholder) => (
                <div key={placeholder.id} className={cn('bg-card flex min-h-48 min-w-0 flex-col p-3', placeholder.className)}>
                    <DashboardPanelLoading label={t('loading')} />
                </div>
            ))}
        </>
    )
}
