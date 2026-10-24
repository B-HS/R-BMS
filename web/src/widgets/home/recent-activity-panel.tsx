'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { ScoreRow } from '@features/score/score-row'
import { useActivityRecent } from '@entities/stats.query'
import { DASHBOARD_FULL_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { formatChartTitle } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

const RECENT_LIMIT = 20

export const RecentActivityPanel: FC = () => {
    const t = useTranslations('browse.home.recent')
    const { data, isLoading, isError, isFetching, refetch } = useActivityRecent({ limit: RECENT_LIMIT })
    const rows = data ?? []

    return (
        <DashboardPanel title={t('title')} className={DASHBOARD_FULL_PANEL_CLASS_NAME} bodyClassName='px-0 pb-0'>
            {isLoading && <DashboardPanelLoading label={t('loading')} className='px-3 pb-3' />}
            {!isLoading && isError && <DashboardPanelError message={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && rows.length === 0 && (
                <DashboardPanelEmpty message={t('empty')}>
                    <Button asChild variant='outline' size='xs'>
                        <Link href='/guide'>{t('guideButton')}</Link>
                    </Button>
                </DashboardPanelEmpty>
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <ul aria-label={t('listLabel')} className='border-border flex flex-col border-t'>
                    {rows.map((row) => (
                        <ScoreRow
                            key={row.score_id}
                            href={`/charts/${row.chart.sha256}`}
                            title={formatChartTitle(row.chart)}
                            subtitle={row.player_name}
                            subtitleHref={`/players/${encodeURIComponent(row.player.id)}`}
                            lamp={row.clear}
                            assistGrade={row.assist_grade}
                            exScore={row.ex_score}
                            minBp={row.minbp}
                            playedAt={row.played_at}
                        />
                    ))}
                </ul>
            )}
            <p className='text-muted-foreground px-3 py-2 text-xs'>{t('note')}</p>
        </DashboardPanel>
    )
}
