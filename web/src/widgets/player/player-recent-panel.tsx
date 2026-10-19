'use client'

import type { FC } from 'react'
import { usePathname, useRouter } from 'next/navigation'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { ScoreRow } from '@features/score/score-row'
import { Pager } from '@features/table/pager'
import { usePlayerRecent } from '@entities/player.query'
import type { PlayerRecentParams } from '@entities/player.type'
import { formatChartTitle, formatCount } from '@shared/lib/format'
import { serializeSearchParams } from '@shared/lib/search-params'
import { Button } from '@shared/ui/button'

const FIRST_PAGE = 1

type PlayerRecentPanelProps = {
    playerId: string
    params: PlayerRecentParams
}

export const PlayerRecentPanel: FC<PlayerRecentPanelProps> = ({ playerId, params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const { data, isLoading, isError, isFetching, refetch } = usePlayerRecent(playerId, params)
    const rows = data?.data ?? []
    const total = data?.pagination?.total ?? 0
    const totalPages = data?.pagination?.totalPages ?? FIRST_PAGE

    const handlePageChange = (page: number) => {
        const query = serializeSearchParams({ page: page > FIRST_PAGE ? page : undefined })
        router.push(query ? `${pathname}?${query}` : pathname, { scroll: false })
    }

    return (
        <DashboardPanel
            title='플레이 기록'
            badge={total > 0 && <span className='text-muted-foreground text-2xs tabular-nums'>{formatCount(total)}판</span>}
            bodyClassName='px-0 pb-0'>
            {isLoading && <DashboardPanelLoading label='플레이 기록 불러오는 중' className='px-3 pb-3' />}
            {!isLoading && isError && (
                <DashboardPanelError message='플레이 기록을 불러오지 못했습니다.' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {!isLoading && !isError && total === 0 && <DashboardPanelEmpty message='아직 기록이 없습니다.' />}
            {!isLoading && !isError && total > 0 && rows.length === 0 && (
                <DashboardPanelEmpty message='이 페이지에는 기록이 없습니다.'>
                    <Button variant='outline' size='xs' onClick={() => handlePageChange(totalPages)}>
                        마지막 페이지로
                    </Button>
                </DashboardPanelEmpty>
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <ul aria-label='플레이어 플레이 기록' className='border-border flex flex-col border-t'>
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
                            isScratchAuto={row.scratch_auto}
                        />
                    ))}
                </ul>
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <div className='px-3'>
                    <Pager page={params.page} totalPages={totalPages} onPageChange={handlePageChange} />
                </div>
            )}
            <p className='text-muted-foreground px-3 py-2 text-xs'>
                순위 반영 여부와 무관하게 모든 판을 최신순으로 표시합니다. A-SCR 은 SCRATCH AUTO 로 친 기록입니다.
            </p>
        </DashboardPanel>
    )
}
