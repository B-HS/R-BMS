'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiChartCandidateList } from '@features/ai/ai-chart-candidate-list'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Button } from '@shared/ui/button'
import { useAiGoalPlan, useCreateAiPracticeFolder, useInterpretAiGoalPlan } from '@entities/ai.query'

type AiGoalPlanSectionProps = { goalId: string; symbol: string }

export const AiGoalPlanSection: FC<AiGoalPlanSectionProps> = ({ goalId, symbol }) => {
    const t = useTranslations('ai.goalPlan')
    const plan = useAiGoalPlan(goalId)
    const interpret = useInterpretAiGoalPlan()
    const createFolder = useCreateAiPracticeFolder()

    const view = plan.data
    const ai = interpret.data ?? view?.ai ?? null
    const titles = new Map((view?.remaining ?? []).map((chart) => [chart.md5, chart.title]))

    return (
        <div className='flex flex-col gap-3 pt-2'>
            {plan.isPending && <PanelSkeleton label={t('loading')} />}
            {plan.isError && <LoadErrorState title={t('loadError')} isRetrying={plan.isFetching} onRetry={() => void plan.refetch()} />}
            {view && (
                <>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {view.remaining.length > 0 && <AiChartCandidateList label={t('remainingLabel')} symbol={symbol} candidates={view.remaining} />}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({ goalId })}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && (
                            <ol aria-label={t('stepsLabel')} className='flex flex-col gap-2'>
                                {ai.output.steps.map((step) => (
                                    <li key={step.week_index} className='border-border flex flex-col gap-1 border p-2 text-xs'>
                                        <span className='text-sm font-medium'>{t('stepHeading', { week: step.week_index, focus: step.focus })}</span>
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
                                                            name_suffix: t('folderNameSuffix', { week: step.week_index }),
                                                            description: step.focus,
                                                            items: step.chart_md5s.map((md5) => ({ md5, memo: '' })),
                                                        })
                                                    }>
                                                    {t('createFolder')}
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
