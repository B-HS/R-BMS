'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { LampDistribution } from '@features/lamp/lamp-distribution'
import { StatTile } from '@features/shell/stat-tile'
import { PlayerPlayDays } from '@widgets/player/player-play-days'
import { usePlayerStats } from '@entities/player.query'
import { DISPLAY_LAMPS, isSameDisplayLamp } from '@shared/constants/clear-lamp'
import { formatCount, formatPercent } from '@shared/lib/format'
import { ToggleGroup, ToggleGroupItem } from '@shared/ui/toggle-group'

const NO_PLAY_LAMP = 'NoPlay'

const PLAYER_STATS_MODE = { LAMP: 'lamp', PLAY_DAYS: 'play-days' } as const

type PlayerStatsMode = (typeof PLAYER_STATS_MODE)[keyof typeof PLAYER_STATS_MODE]

export const PlayerStatsPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const [mode, setMode] = useState<PlayerStatsMode>(PLAYER_STATS_MODE.LAMP)
    const t = useTranslations('browse.player.stats')
    const tPlayDays = useTranslations('browse.player.playDays')
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

    const handleModeChange = (value: string) => {
        const next = Object.values(PLAYER_STATS_MODE).find((candidate) => candidate === value)
        if (next) setMode(next)
    }

    const modeToggle = (
        <ToggleGroup
            type='single'
            variant='outline'
            size='sm'
            spacing={0}
            aria-label={tPlayDays('modeLabel')}
            value={mode}
            onValueChange={handleModeChange}>
            <ToggleGroupItem value={PLAYER_STATS_MODE.LAMP} className='px-2 text-xs'>
                {t('lampTitle')}
            </ToggleGroupItem>
            <ToggleGroupItem value={PLAYER_STATS_MODE.PLAY_DAYS} className='px-2 text-xs'>
                {tPlayDays('title')}
            </ToggleGroupItem>
        </ToggleGroup>
    )

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
            <DashboardPanel title={mode === PLAYER_STATS_MODE.LAMP ? t('lampTitle') : tPlayDays('title')} actions={modeToggle}>
                {mode === PLAYER_STATS_MODE.LAMP && (
                    <LampDistribution
                        lamps={lamps}
                        played={data.cleared}
                        total={playedCharts}
                        playedLabel={t('clearedLabel')}
                        rateLabel={t('clearRate')}
                    />
                )}
                {mode === PLAYER_STATS_MODE.PLAY_DAYS && <PlayerPlayDays playerId={playerId} />}
            </DashboardPanel>
        </>
    )
}
