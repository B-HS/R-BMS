'use client'

import type { FC } from 'react'
import { usePathname, useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardPanelError } from '@features/dashboard/dashboard-panel-error'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { ScoreRow } from '@features/score/score-row'
import { Pager } from '@features/table/pager'
import { aiPlayHref } from '@entities/ai.type'
import { useSessionUser } from '@entities/auth.query'
import { usePlayerRecent } from '@entities/player.query'
import type { PlayerRecentParams } from '@entities/player.type'
import { formatChartTitle } from '@shared/lib/format'
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
    const t = useTranslations('browse.player.recent')
    const { data, isLoading, isError, isFetching, refetch } = usePlayerRecent(playerId, params)
    const { user } = useSessionUser()
    const rows = data?.data ?? []
    const total = data?.pagination?.total ?? 0
    const totalPages = data?.pagination?.totalPages ?? FIRST_PAGE
    const isOwnRecord = user?.login_id === playerId

    const handlePageChange = (page: number) => {
        const query = serializeSearchParams({ page: page > FIRST_PAGE ? page : undefined })
        router.push(query ? `${pathname}?${query}` : pathname, { scroll: false })
    }

    return (
        <DashboardPanel
            title={t('title')}
            badge={total > 0 && <span className='text-muted-foreground text-2xs tabular-nums'>{t('count', { count: total })}</span>}
            bodyClassName='px-0 pb-0'>
            {isLoading && <DashboardPanelLoading label={t('loading')} className='px-3 pb-3' />}
            {!isLoading && isError && <DashboardPanelError message={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && total === 0 && <DashboardPanelEmpty message={t('empty')} />}
            {!isLoading && !isError && total > 0 && rows.length === 0 && (
                <DashboardPanelEmpty message={t('pageEmpty')}>
                    <Button variant='outline' size='xs' onClick={() => handlePageChange(totalPages)}>
                        {t('lastPage')}
                    </Button>
                </DashboardPanelEmpty>
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <ul aria-label={t('listLabel')} className='border-border flex flex-col border-t'>
                    {rows.map((row) => (
                        <ScoreRow
                            key={row.score_id}
                            href={`/charts/${row.chart.sha256}`}
                            title={formatChartTitle(row.chart)}
                            subtitle={row.chart.artist || undefined}
                            lamp={row.clear}
                            assistGrade={row.assist_grade}
                            exScore={row.ex_score}
                            minBp={row.minbp}
                            playedAt={row.played_at}
                            isScratchAuto={row.scratch_auto}
                            action={isOwnRecord ? { href: aiPlayHref(row.score_id), label: t('coach') } : undefined}
                        />
                    ))}
                </ul>
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <div className='px-3'>
                    <Pager page={params.page} totalPages={totalPages} onPageChange={handlePageChange} />
                </div>
            )}
            <p className='text-muted-foreground px-3 py-2 text-xs'>{t('note')}</p>
        </DashboardPanel>
    )
}
