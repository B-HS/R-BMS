'use client'

import type { FC } from 'react'
import { AiChartCandidateList } from '@features/ai/ai-chart-candidate-list'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Button } from '@shared/ui/button'
import { useAiGoalPlan, useCreateAiPracticeFolder, useInterpretAiGoalPlan } from '@entities/ai.query'

type AiGoalPlanSectionProps = { goalId: string; symbol: string }

export const AiGoalPlanSection: FC<AiGoalPlanSectionProps> = ({ goalId, symbol }) => {
    const plan = useAiGoalPlan(goalId)
    const interpret = useInterpretAiGoalPlan()
    const createFolder = useCreateAiPracticeFolder()

    const view = plan.data
    const ai = interpret.data ?? view?.ai ?? null
    const titles = new Map((view?.remaining ?? []).map((chart) => [chart.md5, chart.title]))

    return (
        <div className='flex flex-col gap-3 pt-2'>
            {plan.isPending && <PanelSkeleton label='목표 계획 불러오는 중' />}
            {plan.isError && (
                <LoadErrorState title='목표 계획을 불러오지 못했습니다' isRetrying={plan.isFetching} onRetry={() => void plan.refetch()} />
            )}
            {view && (
                <>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {view.remaining.length > 0 && <AiChartCandidateList label='남은 곡(가까운 순)' symbol={symbol} candidates={view.remaining} />}
                    <AiInterpretSection
                        label='AI 목표 계획'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({ goalId })}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && (
                            <ol aria-label='주차별 계획' className='flex flex-col gap-2'>
                                {ai.output.steps.map((step) => (
                                    <li key={step.week_index} className='border-border flex flex-col gap-1 border p-2 text-xs'>
                                        <span className='text-sm font-medium'>
                                            {step.week_index}주차 · {step.focus}
                                        </span>
                                        {step.chart_md5s.length > 0 && (
                                            <span className='text-muted-foreground'>
                                                {step.chart_md5s.map((md5) => titles.get(md5) ?? md5).join(', ')}
                                            </span>
                                        )}
                                        {step.chart_md5s.length > 0 && (
                                            <div>
                                                <Button
                                                    type='button'
                                                    variant='outline'
                                                    size='xs'
                                                    disabled={createFolder.isPending}
                                                    onClick={() =>
                                                        createFolder.mutate({
                                                            source: 'goal',
                                                            goal_id: goalId,
                                                            name_suffix: `목표 ${step.week_index}주차`,
                                                            description: step.focus,
                                                            items: step.chart_md5s.map((md5) => ({ md5, memo: '' })),
                                                        })
                                                    }>
                                                    이 주차를 연습 폴더로 만들기
                                                </Button>
                                            </div>
                                        )}
                                    </li>
                                ))}
                            </ol>
                        )}
                    </AiInterpretSection>
                </>
            )}
        </div>
    )
}
