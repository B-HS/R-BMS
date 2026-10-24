'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiGoalForm } from '@features/ai/ai-goal-form'
import { PanelCard } from '@features/shell/panel-card'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { Progress } from '@shared/ui/progress'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiGoalList, useAiRecommend, useArchiveAiGoal, useCreateAiGoal } from '@entities/ai.query'
import { AI_SCORE_FEATURE_FORM_LIMIT } from '@entities/ai.type'
import { AiGoalPlanSection } from '@widgets/ai/ai-goal-plan-section'

export const AiGoalPanel: FC = () => {
    const [openGoalId, setOpenGoalId] = useState<string | null>(null)

    const t = useTranslations('ai.goal')
    const goals = useAiGoalList()
    const table = useAiRecommend(null)
    const create = useCreateAiGoal()
    const archive = useArchiveAiGoal()

    const symbol = table.data?.table.symbol ?? ''
    const isFull = (goals.data?.length ?? 0) >= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_ACTIVE_MAX

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {table.data && (
                <AiGoalForm
                    tableId={table.data.table.id}
                    symbol={symbol}
                    levels={table.data.levels.map((level) => level.level)}
                    isSubmitting={create.isPending || isFull}
                    onSubmit={(params) => create.mutate(params)}
                />
            )}
            <p className='text-muted-foreground text-xs'>{t('description', { max: AI_SCORE_FEATURE_FORM_LIMIT.GOAL_ACTIVE_MAX })}</p>
            {goals.isPending && <PanelSkeleton label={t('loading')} />}
            {goals.isError && <LoadErrorState title={t('loadError')} isRetrying={goals.isFetching} onRetry={() => void goals.refetch()} />}
            {goals.data?.length === 0 && <EmptyState title={t('emptyTitle')} description={t('emptyDescription')} />}
            {goals.data && goals.data.length > 0 && (
                <ul aria-label={t('listLabel')} className='flex flex-col gap-3'>
                    {goals.data.map((goal) => (
                        <li key={goal.id} className='border-border flex flex-col gap-2 border p-3'>
                            <div className='flex flex-wrap items-center justify-between gap-2'>
                                <span className='text-sm font-medium'>{goal.progress?.label ?? goal.params.kind}</span>
                                <span className='flex items-center gap-2'>
                                    {goal.progress?.done && <Badge>{t('done')}</Badge>}
                                    <Button
                                        type='button'
                                        variant='ghost'
                                        size='xs'
                                        onClick={() => setOpenGoalId(openGoalId === goal.id ? null : goal.id)}>
                                        {openGoalId === goal.id ? t('closePlan') : t('openPlan')}
                                    </Button>
                                    <Button
                                        type='button'
                                        variant='ghost'
                                        size='xs'
                                        disabled={archive.isPending}
                                        onClick={() => archive.mutate(goal.id)}>
                                        {t('archive')}
                                    </Button>
                                </span>
                            </div>
                            {goal.progress && (
                                <div className='flex flex-col gap-1'>
                                    <Progress
                                        value={Math.min(
                                            PERCENT_SCALE,
                                            goal.progress.target > 0 ? (goal.progress.achieved / goal.progress.target) * PERCENT_SCALE : 0,
                                        )}
                                        aria-label={t('progressLabel', { label: goal.progress.label })}
                                    />
                                    <span className='text-muted-foreground text-xs tabular-nums'>
                                        {t('progressText', {
                                            achieved: formatCount(goal.progress.achieved),
                                            target: formatCount(goal.progress.target),
                                            updates: goal.progress.recent_updates,
                                        })}
                                    </span>
                                </div>
                            )}
                            {openGoalId === goal.id && <AiGoalPlanSection goalId={goal.id} symbol={symbol} />}
                        </li>
                    ))}
                </ul>
            )}
        </PanelCard>
    )
}
