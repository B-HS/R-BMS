'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { ScoreRow } from '@features/score/score-row'
import { aiPlayHref } from '@entities/ai.type'
import { usePlayerRecent } from '@entities/player.query'
import { DASHBOARD_RECENT_PLAYS_LIMIT, DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { formatChartTitle } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

const RECENT_PARAMS = { page: 1, limit: DASHBOARD_RECENT_PLAYS_LIMIT }

type MemberRecentPanelProps = {
    loginId: string
    trainingSha256s?: ReadonlySet<string>
}

export const MemberRecentPanel: FC<MemberRecentPanelProps> = ({ loginId, trainingSha256s }) => {
    const t = useTranslations('pages.dashboard.recent')
    const { data, isPending, isError, isFetching, refetch } = usePlayerRecent(loginId, RECENT_PARAMS)
    const rows = data?.data ?? []

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_WIDE_PANEL_CLASS_NAME}
            bodyClassName='px-0 pb-0'
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href={`/players/${encodeURIComponent(loginId)}`}>{t('viewAll')}</Link>
                </Button>
            }>
            <DashboardQueryState
                isPending={isPending}
                isError={isError}
                isFetching={isFetching}
                onRetry={() => void refetch()}
                loadingLabel={t('loading')}
                errorMessage={t('loadError')}
                isEmpty={rows.length === 0}
                emptyMessage={t('empty')}
                loadingClassName='px-3 pb-3'>
                <ul aria-label={t('listLabel')} className='border-border flex max-h-128 flex-col overflow-y-auto border-t *:shrink-0'>
                    {rows.map((row) => (
                        <ScoreRow
                            key={row.score_id}
                            href={`/charts/${row.chart.sha256}`}
                            title={formatChartTitle(row.chart)}
                            subtitle={
                                trainingSha256s?.has(row.chart.sha256) ? `${t('training')} · ${row.chart.artist}` : row.chart.artist || undefined
                            }
                            lamp={row.clear}
                            assistGrade={row.assist_grade}
                            exScore={row.ex_score}
                            minBp={row.minbp}
                            playedAt={row.played_at}
                            isScratchAuto={row.scratch_auto}
                            action={{ href: aiPlayHref(row.score_id), label: t('coach') }}
                        />
                    ))}
                </ul>
            </DashboardQueryState>
            <p className='text-muted-foreground px-3 py-2 text-xs'>{t('note')}</p>
        </DashboardPanel>
    )
}
