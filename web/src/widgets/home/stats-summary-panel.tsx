'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { useStatsSummary } from '@entities/stats.query'
import { DASHBOARD_FILL_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

const ROW_CLASS = 'flex items-baseline justify-between gap-3 text-xs'

export const StatsSummaryPanel: FC = () => {
    const t = useTranslations('browse.home.stats')
    const { data, isLoading, isError, isFetching, refetch } = useStatsSummary()

    return (
        <DashboardPanel title={t('title')} className={DASHBOARD_FILL_PANEL_CLASS_NAME} bodyClassName='gap-2'>
            {isLoading && <DashboardPanelLoading label={t('loading')} />}
            {!isLoading && (isError || !data) && (
                <DashboardPanelError message={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {!isLoading && !isError && data && (
                <>
                    <dl className='flex items-baseline gap-2'>
                        <dt className='text-muted-foreground text-xs'>{t('charts')}</dt>
                        <dd
                            className={cn(
                                'text-2xl leading-8 font-semibold tracking-tight tabular-nums',
                                data.charts === 0 && 'text-muted-foreground',
                            )}>
                            {formatCount(data.charts)}
                        </dd>
                    </dl>
                    <dl className='flex flex-col gap-1'>
                        <div className={ROW_CLASS}>
                            <dt className='text-muted-foreground'>{t('players')}</dt>
                            <dd className='font-medium tabular-nums'>{formatCount(data.players)}</dd>
                        </div>
                        <div className={ROW_CLASS}>
                            <dt className='text-muted-foreground'>{t('submissions')}</dt>
                            <dd className='font-medium tabular-nums'>{formatCount(data.scores)}</dd>
                        </div>
                    </dl>
                </>
            )}
        </DashboardPanel>
    )
}
