'use client'

import type { FC } from 'react'
import { PanelCard } from '@features/shell/panel-card'
import { StatTile } from '@features/shell/stat-tile'
import { BarCountList } from '@features/stats/bar-count-list'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { usePlayerStats } from '@entities/player.query'
import { CLEAR_LAMP_LABEL } from '@shared/constants/clear-lamp'
import { formatCount, formatPercent } from '@shared/lib/format'

export const PlayerStatsPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const { data, isLoading, isError } = usePlayerStats(playerId)

    if (isLoading) return <PanelSkeleton height='table' />
    if (isError) return <LoadErrorState title='통계를 불러오지 못했습니다' />
    if (!data) return null

    const playedCharts = data.lamps.reduce((sum, entry) => sum + entry.count, 0)

    return (
        <>
            <div className='grid grid-cols-2 gap-px md:grid-cols-3'>
                <StatTile label='총 제출' value={formatCount(data.total_plays)} hint='스코어 제출 횟수' />
                <StatTile label='클리어 차트' value={formatCount(data.cleared)} hint={`플레이한 차트 ${formatCount(playedCharts)}개 중`} />
                <StatTile label='클리어 비율' value={formatPercent(data.cleared, playedCharts)} hint='플레이한 차트 기준' />
            </div>
            <PanelCard title='분포'>
                <BarCountList
                    label='clear lamp'
                    items={data.lamps.map((entry) => ({ key: entry.clear, label: CLEAR_LAMP_LABEL[entry.clear], count: entry.count }))}
                />
            </PanelCard>
        </>
    )
}
