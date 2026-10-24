'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInsightList } from '@features/ai/ai-insight-list'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Badge } from '@shared/ui/badge'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiJudgeHabit, useInterpretAiJudgeHabit } from '@entities/ai.query'
import { AI_EVIDENCE_LABEL, AI_GREEN_DIRECTION_LABEL, AI_OFFSET_DIRECTION_LABEL } from '@entities/ai.type'

const MISSING = '—'
const GRADE_LABEL = { great: 'GREAT', good: 'GOOD', bad: 'BAD' } as const

const formatSigned = (value: number) => `${value > 0 ? '+' : ''}${value}`

export const AiJudgeHabitPanel: FC = () => {
    const t = useTranslations('ai.judgeHabit')
    const habit = useAiJudgeHabit()
    const interpret = useInterpretAiJudgeHabit()

    const view = habit.data
    const ai = interpret.data ?? view?.ai ?? null

    const formatGreen = (value: { green: number; lane_ms: number; is_estimated?: boolean } | null) =>
        value ? t(value.is_estimated ? 'greenEstimated' : 'green', { green: value.green, laneMs: value.lane_ms }) : MISSING

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {habit.isPending && <PanelSkeleton label={t('loading')} />}
            {habit.isError && <LoadErrorState title={t('loadError')} isRetrying={habit.isFetching} onRetry={() => void habit.refetch()} />}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>{t('intro', { plays: view.plays, tracePlays: view.trace_timing.plays })}</p>
                    <section aria-label='FAST·SLOW' className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p className='text-sm'>
                            {t('fastShare', {
                                fast: formatCount(view.fast_slow.fast),
                                slow: formatCount(view.fast_slow.slow),
                                share: view.fast_slow.fast_share === null ? MISSING : `${Math.round(view.fast_slow.fast_share * PERCENT_SCALE)}%`,
                            })}{' '}
                            <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.fast_slow.grade]}</Badge>
                        </p>
                        <ul className='text-muted-foreground flex flex-wrap gap-3'>
                            {view.fast_slow.by_grade.map((row) => (
                                <li key={row.grade}>{t('gradeRow', { grade: GRADE_LABEL[row.grade], early: row.early, late: row.late })}</li>
                            ))}
                        </ul>
                    </section>
                    <section aria-label={t('offsetLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p className='text-sm'>
                            {t('meanOffset', { mean: view.offset.mean_offset_ms === null ? MISSING : `${view.offset.mean_offset_ms} ms` })} ·{' '}
                            <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.offset.grade]}</Badge>
                        </p>
                        <p>
                            {view.offset.suggested_shift_ms === null
                                ? t('offsetSuggestion', { direction: AI_OFFSET_DIRECTION_LABEL[view.offset.direction] })
                                : t('offsetSuggestionShifted', {
                                      direction: AI_OFFSET_DIRECTION_LABEL[view.offset.direction],
                                      shift: formatSigned(view.offset.suggested_shift_ms),
                                  })}
                        </p>
                        <p className='text-muted-foreground'>
                            {t('offsetCounts', { plays: view.offset.plays, autoAdjustPlays: view.offset.auto_adjust_plays })}
                        </p>
                    </section>
                    <section aria-label={t('traceLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p className='text-sm'>
                            {view.trace_timing.ci_low_ms !== null && view.trace_timing.ci_high_ms !== null
                                ? t('traceMeanInterval', {
                                      mean: view.trace_timing.mean_ms === null ? MISSING : `${view.trace_timing.mean_ms} ms`,
                                      low: view.trace_timing.ci_low_ms,
                                      high: view.trace_timing.ci_high_ms,
                                  })
                                : t('traceMean', { mean: view.trace_timing.mean_ms === null ? MISSING : `${view.trace_timing.mean_ms} ms` })}{' '}
                            <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.trace_timing.grade]}</Badge>
                        </p>
                        {view.trace_timing.lanes.length > 0 && (
                            <ul aria-label={t('lanesLabel')} className='text-muted-foreground flex flex-wrap gap-3'>
                                {view.trace_timing.lanes.map((lane) => (
                                    <li key={lane.key}>
                                        {t('laneRow', { lane: lane.lane, mean: lane.mean_ms, deviation: formatSigned(lane.deviation_ms) })}
                                    </li>
                                ))}
                            </ul>
                        )}
                        <p className='text-muted-foreground'>{t(view.offset.source === 'trace' ? 'offsetFromTrace' : 'offsetFromScores')}</p>
                    </section>
                    <section aria-label={t('avgjudgeLabel')} className='text-xs tabular-nums'>
                        {t('avgjudge', { older: view.avgjudge.older_ms ?? MISSING, newer: view.avgjudge.newer_ms ?? MISSING })}{' '}
                        <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.avgjudge.grade]}</Badge>
                    </section>
                    <section aria-label={t('greenLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p>
                            {t('greenSummary', {
                                latest: formatGreen(view.green.latest),
                                median: formatGreen(view.green.median),
                                constantPlays: view.green.constant_plays,
                            })}
                        </p>
                        {view.green.buckets.length > 0 && (
                            <ul aria-label={t('bucketsLabel')} className='text-muted-foreground flex flex-wrap gap-3'>
                                {view.green.buckets.map((bucket) => (
                                    <li key={bucket.key}>
                                        {t('bucketRow', {
                                            from: formatGreen(bucket.from),
                                            plays: bucket.plays,
                                            share: bucket.fast_share === null ? MISSING : `${Math.round(bucket.fast_share * PERCENT_SCALE)}%`,
                                        })}
                                    </li>
                                ))}
                            </ul>
                        )}
                    </section>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && (
                            <p className='text-xs break-keep'>
                                {t('aiOffset', {
                                    direction: AI_OFFSET_DIRECTION_LABEL[ai.output.offset_direction],
                                    reasoning: ai.output.offset_reasoning,
                                })}
                            </p>
                        )}
                        {ai && (
                            <p className='text-xs break-keep'>
                                {t('aiGreen', {
                                    direction: AI_GREEN_DIRECTION_LABEL[ai.output.green_direction],
                                    reasoning: ai.output.green_reasoning,
                                })}
                            </p>
                        )}
                        {ai && ai.output.notes.length > 0 && <AiInsightList label={t('notesLabel')} insights={ai.output.notes} />}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
