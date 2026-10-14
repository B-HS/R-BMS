'use client'

import type { FC } from 'react'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { useStatsSummary } from '@entities/stats.query'
import { DASHBOARD_FILL_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

const ROW_CLASS = 'flex items-baseline justify-between gap-3 text-xs'

export const StatsSummaryPanel: FC = () => {
    const { data, isLoading, isError, isFetching, refetch } = useStatsSummary()

    return (
        <DashboardPanel title='서버 현황' className={DASHBOARD_FILL_PANEL_CLASS_NAME} bodyClassName='gap-2'>
            {isLoading && <DashboardPanelLoading label='서버 현황 불러오는 중' />}
            {!isLoading && (isError || !data) && (
                <DashboardPanelError message='서버 현황을 불러오지 못했습니다.' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {!isLoading && !isError && data && (
                <>
                    <dl className='flex items-baseline gap-2'>
                        <dt className='text-muted-foreground text-xs'>등록 차트</dt>
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
                            <dt className='text-muted-foreground'>플레이어</dt>
                            <dd className='font-medium tabular-nums'>{formatCount(data.players)}</dd>
                        </div>
                        <div className={ROW_CLASS}>
                            <dt className='text-muted-foreground'>누적 제출</dt>
                            <dd className='font-medium tabular-nums'>{formatCount(data.scores)}</dd>
                        </div>
                    </dl>
                </>
            )}
        </DashboardPanel>
    )
}
