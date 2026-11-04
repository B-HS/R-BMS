'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { StatTileGrid } from '@features/dashboard/stat-tile-grid'
import { useDashboardOverview } from '@entities/dashboard.query'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { formatCount, formatHoursMinutes, formatPercent } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

export const MemberSummaryPanel: FC = () => {
    const t = useTranslations('pages.dashboard.summary')
    const { data, isPending, isError, isFetching, refetch } = useDashboardOverview()
    const tiles = data?.tiles
    const items = tiles
        ? [
              { key: 'totalPlays', label: t('totalPlays'), value: formatCount(tiles.total_plays) },
              { key: 'playedCharts', label: t('playedCharts'), value: formatCount(tiles.played_charts) },
              {
                  key: 'clearedCharts',
                  label: t('clearedCharts'),
                  value: formatCount(tiles.cleared_charts),
                  hint: t('clearRate', { rate: formatPercent(tiles.cleared_charts, tiles.played_charts) }),
              },
              { key: 'hardOrBetter', label: t('hardOrBetter'), value: formatCount(tiles.hard_or_better) },
              { key: 'fullComboOrBetter', label: t('fullComboOrBetter'), value: formatCount(tiles.full_combo_or_better) },
              {
                  key: 'playTime',
                  label: t('playTime'),
                  value: formatHoursMinutes(tiles.play_ms_30d),
                  hint: t('activeDays', { days: tiles.active_days_30d }),
              },
              {
                  key: 'streak',
                  label: t('streak'),
                  value: t('days', { days: tiles.streak_days }),
                  hint: t('bestStreak', { days: tiles.best_streak_days }),
              },
              { key: 'thisWeek', label: t('thisWeek'), value: formatCount(tiles.this_week_plays) },
          ]
        : []

    return (
        <DashboardPanel title={t('title')} className={DASHBOARD_WIDE_PANEL_CLASS_NAME}>
            <DashboardQueryState
                isPending={isPending}
                isError={isError}
                isFetching={isFetching}
                onRetry={() => void refetch()}
                loadingLabel={t('loading')}
                errorMessage={t('loadError')}
                isEmpty={tiles?.total_plays === 0}
                emptyMessage={t('empty')}
                emptyAction={
                    <Button asChild variant='outline' size='xs'>
                        <Link href='/guide'>{t('guideButton')}</Link>
                    </Button>
                }>
                <StatTileGrid label={t('title')} items={items} />
            </DashboardQueryState>
        </DashboardPanel>
    )
}
