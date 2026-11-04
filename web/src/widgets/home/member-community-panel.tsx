'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { ScoreRow } from '@features/score/score-row'
import { useActivityRecent, useStatsSummary } from '@entities/stats.query'
import { DASHBOARD_COMMUNITY_ACTIVITY_LIMIT, DASHBOARD_FULL_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { formatChartTitle, formatCount } from '@shared/lib/format'

export const MemberCommunityPanel: FC = () => {
    const t = useTranslations('pages.dashboard.community')
    const stats = useStatsSummary()
    const activity = useActivityRecent({ limit: DASHBOARD_COMMUNITY_ACTIVITY_LIMIT })
    const rows = activity.data ?? []

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_FULL_PANEL_CLASS_NAME}
            bodyClassName='px-0 pb-0'
            badge={
                stats.data && (
                    <span className='text-muted-foreground text-2xs tabular-nums'>
                        {t('serverLine', {
                            charts: formatCount(stats.data.charts),
                            players: formatCount(stats.data.players),
                            scores: formatCount(stats.data.scores),
                        })}
                    </span>
                )
            }>
            <DashboardQueryState
                isPending={activity.isPending}
                isError={activity.isError}
                isFetching={activity.isFetching}
                onRetry={() => void activity.refetch()}
                loadingLabel={t('loading')}
                errorMessage={t('loadError')}
                isEmpty={rows.length === 0}
                emptyMessage={t('empty')}
                loadingClassName='mx-3 mb-3 min-h-40'>
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
                            isScratchAuto={row.scratch_auto}
                        />
                    ))}
                </ul>
            </DashboardQueryState>
            <p className='text-muted-foreground px-3 py-2 text-xs'>{t('note')}</p>
        </DashboardPanel>
    )
}
