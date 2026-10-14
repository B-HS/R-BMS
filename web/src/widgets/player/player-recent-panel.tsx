'use client'

import type { FC } from 'react'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { ScoreRow } from '@features/score/score-row'
import { usePlayerRecent } from '@entities/player.query'
import { formatChartTitle } from '@shared/lib/format'

const RECENT_LIMIT = 20

export const PlayerRecentPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const { data, isLoading, isError, isFetching, refetch } = usePlayerRecent(playerId, { limit: RECENT_LIMIT })
    const rows = data ?? []

    return (
        <DashboardPanel title='최근 기록' bodyClassName='px-0 pb-0'>
            {isLoading && <DashboardPanelLoading label='최근 기록 불러오는 중' className='px-3 pb-3' />}
            {!isLoading && isError && (
                <DashboardPanelError message='최근 기록을 불러오지 못했습니다.' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {!isLoading && !isError && rows.length === 0 && <DashboardPanelEmpty message='아직 기록이 없습니다.' />}
            {!isLoading && !isError && rows.length > 0 && (
                <ul aria-label='플레이어 최근 기록' className='border-border flex flex-col border-t'>
                    {rows.map((row) => (
                        <ScoreRow
                            key={row.score_id}
                            href={`/charts/${row.chart.sha256}`}
                            title={formatChartTitle(row.chart)}
                            subtitle={row.chart.artist || undefined}
                            lamp={row.clear}
                            exScore={row.ex_score}
                            minBp={row.minbp}
                            playedAt={row.played_at}
                        />
                    ))}
                </ul>
            )}
            <p className='text-muted-foreground px-3 py-2 text-xs'>순위 반영 여부와 무관하게 최근 제출 순으로 표시됩니다.</p>
        </DashboardPanel>
    )
}
