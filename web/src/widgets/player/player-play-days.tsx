'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { PlayDayChart } from '@features/stats/play-day-chart'
import { PlayDayTable } from '@features/stats/play-day-table'
import { usePlayerPlayDays } from '@entities/player.query'
import { formatHoursMinutes } from '@shared/lib/format'

export const PlayerPlayDays: FC<{ playerId: string }> = ({ playerId }) => {
    const t = useTranslations('browse.player.playDays')
    const { data, isLoading, isError, isFetching, refetch } = usePlayerPlayDays(playerId)

    if (isLoading) return <DashboardPanelLoading label={t('loading')} />

    if (isError || !data) return <DashboardPanelError message={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />

    const dayCount = data.days.length

    if (data.total_plays === 0) return <DashboardPanelEmpty message={t('empty', { days: dayCount })} />

    const chartDays = data.days.map((day) => ({
        date: day.date,
        playMs: day.play_ms,
        label: t('barLabel', { date: day.date, time: formatHoursMinutes(day.play_ms) }),
    }))
    const playedDays = data.days
        .filter((day) => day.plays > 0)
        .toReversed()
        .map((day) => ({ date: day.date, plays: day.plays, playMs: day.play_ms }))

    return (
        <div className='flex min-w-0 flex-col gap-3'>
            <PlayDayChart days={chartDays} listLabel={t('chartLabel', { days: dayCount })} />
            <p className='text-sm font-medium tabular-nums'>{t('total', { days: dayCount, time: formatHoursMinutes(data.total_ms) })}</p>
            <PlayDayTable rows={playedDays} labels={{ date: t('columnDate'), plays: t('columnPlays'), time: t('columnTime') }} />
            <p className='text-muted-foreground text-xs'>{t('note')}</p>
        </div>
    )
}
