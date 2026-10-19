'use client'

import { useState, type FC } from 'react'
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

    const goals = useAiGoalList()
    const table = useAiRecommend(null)
    const create = useCreateAiGoal()
    const archive = useArchiveAiGoal()

    const symbol = table.data?.table.symbol ?? ''
    const isFull = (goals.data?.length ?? 0) >= AI_SCORE_FEATURE_FORM_LIMIT.GOAL_ACTIVE_MAX

    return (
        <PanelCard title='목표 계획' contentClassName='flex flex-col gap-4'>
            {table.data && (
                <AiGoalForm
                    tableId={table.data.table.id}
                    symbol={symbol}
                    levels={table.data.levels.map((level) => level.level)}
                    isSubmitting={create.isPending || isFull}
                    onSubmit={(params) => create.mutate(params)}
                />
            )}
            <p className='text-muted-foreground text-xs'>
                목표는 정해진 종류에서만 고릅니다. 진행도와 남은 곡은 서버가 계산하고, AI 는 주차별 순서와 초점만 정합니다. 활성 목표는{' '}
                {AI_SCORE_FEATURE_FORM_LIMIT.GOAL_ACTIVE_MAX}개까지입니다.
            </p>
            {goals.isPending && <PanelSkeleton label='목표 불러오는 중' />}
            {goals.isError && (
                <LoadErrorState title='목표를 불러오지 못했습니다' isRetrying={goals.isFetching} onRetry={() => void goals.refetch()} />
            )}
            {goals.data?.length === 0 && <EmptyState title='아직 목표가 없습니다' description='위에서 목표를 하나 정해 보세요.' />}
            {goals.data && goals.data.length > 0 && (
                <ul aria-label='내 목표' className='flex flex-col gap-3'>
                    {goals.data.map((goal) => (
                        <li key={goal.id} className='border-border flex flex-col gap-2 border p-3'>
                            <div className='flex flex-wrap items-center justify-between gap-2'>
                                <span className='text-sm font-medium'>{goal.progress?.label ?? goal.params.kind}</span>
                                <span className='flex items-center gap-2'>
                                    {goal.progress?.done && <Badge>달성</Badge>}
                                    <Button
                                        type='button'
                                        variant='ghost'
                                        size='xs'
                                        onClick={() => setOpenGoalId(openGoalId === goal.id ? null : goal.id)}>
                                        {openGoalId === goal.id ? '계획 닫기' : '계획 보기'}
                                    </Button>
                                    <Button
                                        type='button'
                                        variant='ghost'
                                        size='xs'
                                        disabled={archive.isPending}
                                        onClick={() => archive.mutate(goal.id)}>
                                        보관
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
                                        aria-label={`${goal.progress.label} 진행도`}
                                    />
                                    <span className='text-muted-foreground text-xs tabular-nums'>
                                        {formatCount(goal.progress.achieved)} / {formatCount(goal.progress.target)} · 최근 4주 갱신{' '}
                                        {formatCount(goal.progress.recent_updates)}곡
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
