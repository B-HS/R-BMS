'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { RivalVersusList } from '@features/dashboard/rival-versus-list'
import { ScoreRow } from '@features/score/score-row'
import { useDashboardRivals } from '@entities/dashboard.query'
import { DASHBOARD_RIVALS_LIMIT } from '@entities/dashboard.type'
import { DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { useInView } from '@shared/hooks/use-in-view'
import { formatChartTitle } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

export const MemberRivalPanel: FC = () => {
    const t = useTranslations('pages.dashboard.rival')
    const { ref, isInView } = useInView()
    const { data, isPending, isError, isFetching, refetch } = useDashboardRivals(DASHBOARD_RIVALS_LIMIT, isInView)
    const names = new Map(data?.rivals.map((rival) => [rival.login_id, rival.name] as const))

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_WIDE_PANEL_CLASS_NAME}
            bodyClassName='px-0 pb-0'
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href='/settings?tab=rivals'>{t('manage')}</Link>
                </Button>
            }>
            <div ref={ref} className='flex min-h-0 flex-1 flex-col'>
                <DashboardQueryState
                    isPending={isPending}
                    isError={isError}
                    isFetching={isFetching}
                    onRetry={() => void refetch()}
                    loadingLabel={t('loading')}
                    errorMessage={t('loadError')}
                    isEmpty={data?.rivals.length === 0}
                    emptyMessage={t('empty')}
                    emptyAction={
                        <Button asChild variant='outline' size='xs'>
                            <Link href='/settings?tab=rivals'>{t('add')}</Link>
                        </Button>
                    }
                    loadingClassName='mx-3 mb-3 min-h-32'>
                    {data && (
                        <div className='@container'>
                            <div className='grid min-w-0 gap-3 @xl:grid-cols-[minmax(0,1fr)_minmax(0,1.6fr)]'>
                                <section className='flex min-w-0 flex-col gap-1.5 px-3'>
                                    <h3 className='rbms-micro-label'>{t('versusTitle')}</h3>
                                    <RivalVersusList
                                        label={t('versusLabel')}
                                        noCommonLabel={t('noCommon')}
                                        rows={data.versus.map((row) => ({ ...row, name: names.get(row.login_id) ?? row.login_id }))}
                                        describeRow={(row) =>
                                            t('versusDescription', {
                                                name: row.name,
                                                wins: row.wins,
                                                draws: row.draws,
                                                losses: row.losses,
                                                common: row.common,
                                            })
                                        }
                                    />
                                    <p className='text-muted-foreground text-2xs'>{t('versusNote')}</p>
                                </section>
                                <section className='flex min-w-0 flex-col gap-1.5'>
                                    <h3 className='rbms-micro-label px-3'>{t('recentTitle')}</h3>
                                    {data.recent.length === 0 ? (
                                        <DashboardPanelEmpty message={t('recentEmpty')} />
                                    ) : (
                                        <ul
                                            aria-label={t('recentLabel')}
                                            className='border-border flex max-h-96 flex-col overflow-y-auto border-t *:shrink-0'>
                                            {data.recent.map((row) => (
                                                <ScoreRow
                                                    key={row.score_id}
                                                    href={`/charts/${row.chart.sha256}`}
                                                    title={formatChartTitle(row.chart)}
                                                    subtitle={row.player_name}
                                                    subtitleHref={`/players/${encodeURIComponent(row.player.id)}`}
                                                    lamp={row.clear}
                                                    assistGrade={row.assist_grade}
                                                    exScore={row.ex_score}
                                                    minBp={row.minbp}
                                                    playedAt={row.played_at}
                                                    isScratchAuto={row.scratch_auto}
                                                />
                                            ))}
                                        </ul>
                                    )}
                                </section>
                            </div>
                        </div>
                    )}
                </DashboardQueryState>
            </div>
        </DashboardPanel>
    )
}
