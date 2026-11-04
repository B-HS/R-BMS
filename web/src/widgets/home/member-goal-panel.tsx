'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useQuery } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { aiGoalListQueryOptions } from '@entities/ai.query'
import { DASHBOARD_FILL_PANEL_CLASS_NAME, DASHBOARD_GOALS_SHOWN } from '@shared/constants/dashboard'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useInView } from '@shared/hooks/use-in-view'
import { formatCount } from '@shared/lib/format'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { Progress } from '@shared/ui/progress'

const progressPercentOf = (progress: { achieved: number; target: number }) =>
    Math.min(PERCENT_SCALE, progress.target > 0 ? (progress.achieved / progress.target) * PERCENT_SCALE : 0)

export const MemberGoalPanel: FC = () => {
    const t = useTranslations('pages.dashboard.goal')
    const { ref, isInView } = useInView()
    const { data, isPending, isError, isFetching, refetch } = useQuery({ ...aiGoalListQueryOptions(), enabled: isInView })
    const goals = data ?? []

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_FILL_PANEL_CLASS_NAME}
            actions={
                <Button asChild variant='ghost' size='xs'>
                    <Link href='/ai?tab=goals'>{t('manage')}</Link>
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
                    isEmpty={goals.length === 0}
                    emptyMessage={t('empty')}
                    emptyAction={
                        <Button asChild variant='outline' size='xs'>
                            <Link href='/ai?tab=goals'>{t('create')}</Link>
                        </Button>
                    }>
                    <ul aria-label={t('listLabel')} className='flex flex-col gap-3'>
                        {goals.slice(0, DASHBOARD_GOALS_SHOWN).map((goal) => (
                            <li key={goal.id} className='flex flex-col gap-1'>
                                <div className='flex items-baseline justify-between gap-2 text-xs'>
                                    <span className='min-w-0 truncate'>{goal.progress?.label ?? goal.params.kind}</span>
                                    {goal.progress?.done && <Badge variant='outline'>{t('done')}</Badge>}
                                    {goal.progress && !goal.progress.done && (
                                        <span className='text-muted-foreground shrink-0 tabular-nums'>
                                            {formatCount(goal.progress.achieved)} / {formatCount(goal.progress.target)}
                                        </span>
                                    )}
                                </div>
                                {goal.progress && (
                                    <Progress
                                        value={progressPercentOf(goal.progress)}
                                        aria-label={t('progressLabel', { label: goal.progress.label })}
                                    />
                                )}
                            </li>
                        ))}
                    </ul>
                    {goals.length > DASHBOARD_GOALS_SHOWN && (
                        <p className='text-muted-foreground text-2xs'>{t('more', { count: goals.length - DASHBOARD_GOALS_SHOWN })}</p>
                    )}
                </DashboardQueryState>
            </div>
        </DashboardPanel>
    )
}
