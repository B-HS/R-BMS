'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { LampDistribution } from '@features/lamp/lamp-distribution'
import { StatTile } from '@features/shell/stat-tile'
import { usePlayerStats } from '@entities/player.query'
import { DISPLAY_LAMPS, isSameDisplayLamp } from '@shared/constants/clear-lamp'
import { formatCount, formatPercent } from '@shared/lib/format'

const NO_PLAY_LAMP = 'NoPlay'

export const PlayerStatsPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const t = useTranslations('browse.player.stats')
    const { data, isLoading, isError, isFetching, refetch } = usePlayerStats(playerId)

    if (isLoading)
        return (
            <DashboardPanel title={t('lampTitle')}>
                <DashboardPanelLoading label={t('loading')} />
            </DashboardPanel>
        )

    if (isError || !data)
        return (
            <DashboardPanel title={t('lampTitle')}>
                <DashboardPanelError message={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
            </DashboardPanel>
        )

    const playedCharts = data.lamps.reduce((sum, entry) => sum + entry.count, 0)
    const lamps = DISPLAY_LAMPS.filter(({ lamp }) => lamp !== NO_PLAY_LAMP)
        .toReversed()
        .map((displayed) => ({
            ...displayed,
            count: data.lamps.find((entry) => isSameDisplayLamp(displayed, { lamp: entry.clear, assistGrade: entry.assist_grade }))?.count ?? 0,
        }))

    return (
        <>
            <div className='grid grid-cols-2 gap-px md:grid-cols-3'>
                <StatTile label={t('totalPlays')} value={formatCount(data.total_plays)} hint={t('totalPlaysHint')} />
                <StatTile
                    label={t('clearedCharts')}
                    value={formatCount(data.cleared)}
                    hint={
                        data.scratch_auto_charts > 0
                            ? t('clearedHintWithScratchAuto', { played: playedCharts, scratchAuto: data.scratch_auto_charts })
                            : t('clearedHint', { played: playedCharts })
                    }
                />
                <StatTile label={t('clearRate')} value={formatPercent(data.cleared, playedCharts)} hint={t('clearRateHint')} />
            </div>
            <DashboardPanel title={t('lampTitle')}>
                <LampDistribution
                    lamps={lamps}
                    played={data.cleared}
                    total={playedCharts}
                    playedLabel={t('clearedLabel')}
                    rateLabel={t('clearRate')}
                />
            </DashboardPanel>
        </>
    )
}
