'use client'

import type { FC } from 'react'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { LampDistribution } from '@features/lamp/lamp-distribution'
import { StatTile } from '@features/shell/stat-tile'
import { usePlayerStats } from '@entities/player.query'
import { CLEAR_LAMPS } from '@shared/constants/clear-lamp'
import { formatCount, formatPercent } from '@shared/lib/format'

const NO_PLAY_LAMP = 'NoPlay'

export const PlayerStatsPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const { data, isLoading, isError, isFetching, refetch } = usePlayerStats(playerId)

    if (isLoading)
        return (
            <DashboardPanel title='램프 분포'>
                <DashboardPanelLoading label='통계 불러오는 중' />
            </DashboardPanel>
        )

    if (isError || !data)
        return (
            <DashboardPanel title='램프 분포'>
                <DashboardPanelError message='통계를 불러오지 못했습니다.' isRetrying={isFetching} onRetry={() => void refetch()} />
            </DashboardPanel>
        )

    const playedCharts = data.lamps.reduce((sum, entry) => sum + entry.count, 0)
    const lamps = CLEAR_LAMPS.filter((lamp) => lamp !== NO_PLAY_LAMP)
        .toReversed()
        .map((lamp) => ({ lamp, count: data.lamps.find((entry) => entry.clear === lamp)?.count ?? 0 }))

    return (
        <>
            <div className='grid grid-cols-2 gap-px md:grid-cols-3'>
                <StatTile label='총 제출' value={formatCount(data.total_plays)} hint='스코어 제출 횟수' />
                <StatTile
                    label='클리어 차트'
                    value={formatCount(data.cleared)}
                    hint={`플레이한 차트 ${formatCount(playedCharts)}개 중${data.scratch_auto_charts > 0 ? ` · SCRATCH AUTO 기록 ${formatCount(data.scratch_auto_charts)}곡 포함` : ''}`}
                />
                <StatTile label='클리어 비율' value={formatPercent(data.cleared, playedCharts)} hint='플레이한 차트 기준' />
            </div>
            <DashboardPanel title='램프 분포'>
                <LampDistribution lamps={lamps} played={data.cleared} total={playedCharts} playedLabel='클리어한 차트' rateLabel='클리어 비율' />
            </DashboardPanel>
        </>
    )
}
