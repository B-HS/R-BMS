'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { GradeDistributionChart } from '@features/dashboard/grade-distribution-chart'
import { LampDistribution } from '@features/lamp/lamp-distribution'
import { useDashboardOverview } from '@entities/dashboard.query'
import { usePlayerStats } from '@entities/player.query'
import { DISPLAY_LAMPS, isSameDisplayLamp } from '@shared/constants/clear-lamp'

const NO_PLAY_LAMP = 'NoPlay'

export const MemberLampPanel: FC<{ loginId: string }> = ({ loginId }) => {
    const t = useTranslations('pages.dashboard.lamp')
    const stats = usePlayerStats(loginId)
    const overview = useDashboardOverview()
    const data = stats.data
    const playedCharts = data?.lamps.reduce((sum, entry) => sum + entry.count, 0) ?? 0
    const lamps = DISPLAY_LAMPS.filter(({ lamp }) => lamp !== NO_PLAY_LAMP)
        .toReversed()
        .map((displayed) => ({
            ...displayed,
            count: data?.lamps.find((entry) => isSameDisplayLamp(displayed, { lamp: entry.clear, assistGrade: entry.assist_grade }))?.count ?? 0,
        }))

    return (
        <DashboardPanel title={t('title')} bodyClassName='gap-4'>
            <DashboardQueryState
                isPending={stats.isPending}
                isError={stats.isError}
                isFetching={stats.isFetching}
                onRetry={() => void stats.refetch()}
                loadingLabel={t('loading')}
                errorMessage={t('loadError')}
                isEmpty={playedCharts === 0}
                emptyMessage={t('empty')}>
                {data && (
                    <>
                        <LampDistribution
                            lamps={lamps}
                            played={data.cleared}
                            total={playedCharts}
                            playedLabel={t('clearedLabel')}
                            rateLabel={t('clearRate')}
                        />
                        {data.scratch_auto_charts > 0 && (
                            <p className='text-muted-foreground text-2xs'>{t('scratchAuto', { count: data.scratch_auto_charts })}</p>
                        )}
                        <section className='flex flex-col gap-1.5'>
                            <h3 className='rbms-micro-label'>{t('gradeTitle')}</h3>
                            <DashboardQueryState
                                isPending={overview.isPending}
                                isError={overview.isError}
                                isFetching={overview.isFetching}
                                onRetry={() => void overview.refetch()}
                                loadingLabel={t('gradeLoading')}
                                errorMessage={t('gradeLoadError')}
                                loadingClassName='min-h-28'>
                                {overview.data && <GradeDistributionChart grades={overview.data.grades} label={t('gradeLabel')} />}
                            </DashboardQueryState>
                            <p className='text-muted-foreground text-2xs'>{t('gradeNote')}</p>
                        </section>
                    </>
                )}
            </DashboardQueryState>
        </DashboardPanel>
    )
}
