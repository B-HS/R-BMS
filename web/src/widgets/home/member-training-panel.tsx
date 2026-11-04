'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { useAiTrainingFocusLabel, useAiTrainingList } from '@entities/ai.query'
import { aiHubHref, aiTrainingHref } from '@entities/ai.type'
import { DASHBOARD_FILL_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useInView } from '@shared/hooks/use-in-view'
import { formatCount } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

const TRAININGS_SHOWN = 5
const EX_DECIMALS = 1

/**
 * The training chart panel of the signed-in home (plan 2026-11-04 8.2): how many of the 20 charts are kept, the newest five with
 * their focus, plays and best EX, and the way to make one. It asks for the list only once it is near the screen.
 */
export const MemberTrainingPanel: FC = () => {
    const t = useTranslations('pages.dashboard.training')
    const { ref, isInView } = useInView()
    const { data, isPending, isError, isFetching, refetch } = useAiTrainingList(isInView)
    const focusLabel = useAiTrainingFocusLabel()
    const charts = data?.charts.slice(0, TRAININGS_SHOWN) ?? []

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_FILL_PANEL_CLASS_NAME}
            badge={
                data && (
                    <span className='text-muted-foreground text-xs tabular-nums'>
                        {t('usage', { used: formatCount(data.used), limit: formatCount(data.limit) })}
                    </span>
                )
            }
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href={aiHubHref('training')}>{t('make')}</Link>
                </Button>
            }>
            <div ref={ref} className='flex min-h-0 flex-1 flex-col gap-2'>
                <DashboardQueryState
                    isPending={isPending}
                    isError={isError}
                    isFetching={isFetching}
                    onRetry={() => void refetch()}
                    loadingLabel={t('loading')}
                    errorMessage={t('loadError')}
                    isEmpty={charts.length === 0}
                    emptyMessage={t('empty')}
                    emptyAction={
                        <Button asChild variant='outline' size='xs'>
                            <Link href={aiHubHref('training')}>{t('make')}</Link>
                        </Button>
                    }>
                    <ul aria-label={t('listLabel')} className='border-border flex flex-col border-t'>
                        {charts.map((chart) => (
                            <li key={chart.id} className='border-border flex items-center justify-between gap-2 border-b py-1.5'>
                                <div className='flex min-w-0 flex-col'>
                                    <Link href={aiTrainingHref(chart.id)} className='truncate text-xs hover:underline'>
                                        {chart.title}
                                    </Link>
                                    <span className='text-muted-foreground text-2xs truncate'>
                                        {chart.focus_keys.length > 0 ? chart.focus_keys.map(focusLabel).join(', ') : t('noFocus')}
                                    </span>
                                </div>
                                <span className='text-muted-foreground text-2xs shrink-0 text-right tabular-nums'>
                                    {t('plays', { count: chart.plays })}
                                    {chart.best_ex_ratio !== null && ` · ${(chart.best_ex_ratio * PERCENT_SCALE).toFixed(EX_DECIMALS)}%`}
                                </span>
                            </li>
                        ))}
                    </ul>
                </DashboardQueryState>
            </div>
        </DashboardPanel>
    )
}
