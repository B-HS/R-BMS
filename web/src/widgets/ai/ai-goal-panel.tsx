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
import { useAiGoalList, useAiProfile, useAiRecommend, useArchiveAiGoal, useCreateAiGoal } from '@entities/ai.query'
import { AI_DENSITY_LIMIT_KEY, AI_SCORE_FEATURE_FORM_LIMIT, formatAiWeaknessValue, isAiResidualGoalKey, type AiGoal } from '@entities/ai.type'
import { AiGoalPlanSection } from '@widgets/ai/ai-goal-plan-section'

const PROFILE_GOAL_UNIT = { weakness_residual: 'ex_residual', density_limit: 'nps' } as const

/** The profile key and unit a profile goal is measured in, or null for the table goals. */
const profileGoalOf = (goal: AiGoal) => {
    if (goal.params.kind === 'weakness_residual')
        return { key: goal.params.key, unit: PROFILE_GOAL_UNIT.weakness_residual, baseline: goal.params.baseline }
    if (goal.params.kind === 'density_limit')
        return { key: AI_DENSITY_LIMIT_KEY, unit: PROFILE_GOAL_UNIT.density_limit, baseline: goal.params.baseline }
    return null
}

const progressPercentOf = (progress: NonNullable<AiGoal['progress']>) => {
    if (progress.ratio !== null && progress.ratio !== undefined) return progress.ratio * PERCENT_SCALE
    return progress.target > 0 ? (progress.achieved / progress.target) * PERCENT_SCALE : 0
}

export const AiGoalPanel: FC = () => {
    const [openGoalId, setOpenGoalId] = useState<string | null>(null)

    const t = useTranslations('ai.goal')
    const goals = useAiGoalList()
    const table = useAiRecommend(null)
    const create = useCreateAiGoal()
    const archive = useArchiveAiGoal()
    const profile = useAiProfile()

    const weaknessKeys = (profile.data?.weaknesses ?? []).map((item) => item.key).filter(isAiResidualGoalKey)
    const symbol = table.data?.table.symbol ?? ''
    const isFull = (goals.data?.length ?? 0) >= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_ACTIVE_MAX

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {table.data && (
                <AiGoalForm
                    tableId={table.data.table.id}
                    symbol={symbol}
                    levels={table.data.levels.map((level) => level.level)}
                    weaknessKeys={weaknessKeys}
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
                    {goals.data.map((goal) => {
                        const profileGoal = profileGoalOf(goal)
                        return (
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
                                            value={Math.min(PERCENT_SCALE, Math.max(0, progressPercentOf(goal.progress)))}
                                            aria-label={t('progressLabel', { label: goal.progress.label })}
                                        />
                                        <span className='text-muted-foreground text-xs tabular-nums'>
                                            {profileGoal
                                                ? t('profileProgressText', {
                                                      current: formatAiWeaknessValue(
                                                          profileGoal.key,
                                                          profileGoal.unit,
                                                          goal.progress.current ?? null,
                                                      ),
                                                      baseline: formatAiWeaknessValue(profileGoal.key, profileGoal.unit, profileGoal.baseline),
                                                      target: formatAiWeaknessValue(profileGoal.key, profileGoal.unit, goal.progress.target),
                                                      updates: goal.progress.recent_updates,
                                                  })
                                                : t('progressText', {
                                                      achieved: formatCount(goal.progress.achieved),
                                                      target: formatCount(goal.progress.target),
                                                      updates: goal.progress.recent_updates,
                                                  })}
                                        </span>
                                    </div>
                                )}
                                {openGoalId === goal.id && <AiGoalPlanSection goalId={goal.id} symbol={symbol} />}
                            </li>
                        )
                    })}
                </ul>
            )}
        </PanelCard>
    )
}
