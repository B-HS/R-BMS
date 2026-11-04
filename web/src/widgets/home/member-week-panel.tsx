'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useQuery } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { StatTileGrid } from '@features/dashboard/stat-tile-grid'
import { aiRecommendQueryOptions, aiWeeklyQueryOptions } from '@entities/ai.query'
import { lampLabel } from '@shared/constants/clear-lamp'
import { DASHBOARD_CHALLENGES_SHOWN } from '@shared/constants/dashboard'
import { useInView } from '@shared/hooks/use-in-view'
import { formatCount } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

const formatDelta = (current: number, previous: number) => {
    const delta = current - previous
    return `${delta > 0 ? '+' : ''}${formatCount(delta)}`
}

export const MemberWeekPanel: FC = () => {
    const t = useTranslations('pages.dashboard.week')
    const { ref, isInView } = useInView()
    const weekly = useQuery({ ...aiWeeklyQueryOptions(null), enabled: isInView })
    const recommend = useQuery({ ...aiRecommendQueryOptions(null), enabled: isInView })
    const view = weekly.data
    const candidates = recommend.data?.candidates.slice(0, DASHBOARD_CHALLENGES_SHOWN) ?? []
    const items = view
        ? [
              {
                  key: 'plays',
                  label: t('plays'),
                  value: formatCount(view.totals.plays),
                  hint: t('versusPrevious', { delta: formatDelta(view.totals.plays, view.previous.plays) }),
              },
              {
                  key: 'activeDays',
                  label: t('activeDays'),
                  value: formatCount(view.totals.active_days),
                  hint: t('versusPrevious', { delta: formatDelta(view.totals.active_days, view.previous.active_days) }),
              },
              {
                  key: 'lampUps',
                  label: t('lampUps'),
                  value: formatCount(view.totals.lamp_ups),
                  hint: t('versusPrevious', { delta: formatDelta(view.totals.lamp_ups, view.previous.lamp_ups) }),
              },
              {
                  key: 'newClears',
                  label: t('newClears'),
                  value: formatCount(view.totals.new_clears),
                  hint: t('versusPrevious', { delta: formatDelta(view.totals.new_clears, view.previous.new_clears) }),
              },
          ]
        : []

    return (
        <DashboardPanel
            title={t('title')}
            bodyClassName='gap-3'
            badge={view?.week.in_progress && <span className='text-muted-foreground text-2xs'>{t('inProgress')}</span>}
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href='/ai?tab=weekly'>{t('viewAll')}</Link>
                </Button>
            }>
            <div ref={ref} className='flex min-h-0 flex-1 flex-col gap-3'>
                <DashboardQueryState
                    isPending={weekly.isPending}
                    isError={weekly.isError}
                    isFetching={weekly.isFetching}
                    onRetry={() => void weekly.refetch()}
                    loadingLabel={t('loading')}
                    errorMessage={t('loadError')}
                    loadingClassName='min-h-28'>
                    {view && (
                        <>
                            <p className='text-muted-foreground text-2xs tabular-nums'>
                                {view.week.start} – {view.week.end}
                            </p>
                            <StatTileGrid label={t('title')} items={items} />
                            {view.notice && <AiDataNotice need={view.notice.need} />}
                        </>
                    )}
                </DashboardQueryState>
                <section className='flex flex-col gap-1.5'>
                    <h3 className='rbms-micro-label'>{t('challengeTitle')}</h3>
                    <DashboardQueryState
                        isPending={recommend.isPending}
                        isError={recommend.isError}
                        isFetching={recommend.isFetching}
                        onRetry={() => void recommend.refetch()}
                        loadingLabel={t('challengeLoading')}
                        errorMessage={t('challengeLoadError')}
                        isEmpty={candidates.length === 0}
                        emptyMessage={t('challengeEmpty')}
                        loadingClassName='min-h-20'>
                        <ul aria-label={t('challengeLabel')} className='border-border flex flex-col border-t'>
                            {candidates.map((candidate) => (
                                <li key={candidate.key} className='border-border flex items-center justify-between gap-2 border-b py-1.5'>
                                    <div className='flex min-w-0 flex-col'>
                                        <Link href={`/charts/${encodeURIComponent(candidate.md5)}`} className='truncate text-xs hover:underline'>
                                            {recommend.data?.table.symbol}
                                            {candidate.level} {candidate.title}
                                        </Link>
                                        <span className='text-muted-foreground text-2xs'>{t(`kind.${candidate.kind}`)}</span>
                                    </div>
                                    <span className='text-muted-foreground shrink-0 text-2xs'>
                                        {lampLabel(candidate.lamp, candidate.assist_grade)}
                                    </span>
                                </li>
                            ))}
                        </ul>
                    </DashboardQueryState>
                </section>
            </div>
        </DashboardPanel>
    )
}
