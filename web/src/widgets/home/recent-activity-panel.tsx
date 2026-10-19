'use client'

import type { FC } from 'react'
import Link from 'next/link'
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
    const { data, isLoading, isError, isFetching, refetch } = useActivityRecent({ limit: RECENT_LIMIT })
    const rows = data ?? []

    return (
        <DashboardPanel title='최근 활동' className={DASHBOARD_FULL_PANEL_CLASS_NAME} bodyClassName='px-0 pb-0'>
            {isLoading && <DashboardPanelLoading label='최근 활동 불러오는 중' className='px-3 pb-3' />}
            {!isLoading && isError && (
                <DashboardPanelError message='최근 활동을 불러오지 못했습니다.' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {!isLoading && !isError && rows.length === 0 && (
                <DashboardPanelEmpty message='아직 올라온 기록이 없습니다.'>
                    <Button asChild variant='outline' size='xs'>
                        <Link href='/guide'>가이드 보기</Link>
                    </Button>
                </DashboardPanelEmpty>
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <ul aria-label='최근 제출 목록' className='border-border flex flex-col border-t'>
                    {rows.map((row) => (
                        <ScoreRow
                            key={row.score_id}
                            href={`/charts/${row.chart.sha256}`}
                            title={formatChartTitle(row.chart)}
                            subtitle={row.player_name}
                            subtitleHref={`/players/${encodeURIComponent(row.player.id)}`}
                            lamp={row.clear}
                            exScore={row.ex_score}
                            minBp={row.minbp}
                            playedAt={row.played_at}
                        />
                    ))}
                </ul>
            )}
            <p className='text-muted-foreground px-3 py-2 text-xs'>
                순위에 반영되는 제출만 표시됩니다. SCRATCH AUTO 판은 이 목록에 나오지 않고, 차트·코스 순위에서 볼 수 있습니다.
            </p>
        </DashboardPanel>
    )
}
