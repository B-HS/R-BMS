'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { AiSegmentChart } from '@features/ai/ai-segment-chart'
import { LampLabel } from '@features/lamp/lamp-label'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { useAiCoach, useInterpretAiCoach } from '@entities/ai.query'
import { AI_GAUGE_LABEL, AI_GAUGE_OUTCOME_LABEL, AI_RUN_END_LABEL, aiPatternTagLabel } from '@entities/ai.type'

const MISSING = '—'
const RATIO_DECIMALS = 1
const MS_PER_SECOND = 1000
const GAUGE_MAX = 100

const formatSeconds = (ms: number) => `${(ms / MS_PER_SECOND).toFixed(RATIO_DECIMALS)}s`

/** Feature 2: one own play's segments (BP bars, gauge line) and its drops first; the AI coaching appears per segment after a press. */
export const AiCoachPanel: FC<{ scoreId: string }> = ({ scoreId }) => {
    const [selectedId, setSelectedId] = useState<string | null>(null)

    const t = useTranslations('ai.coach')
    const coach = useAiCoach(scoreId)
    const interpret = useInterpretAiCoach(scoreId)

    const view = coach.data
    const analysis = view?.analysis ?? null
    const ai = interpret.data ?? view?.ai ?? null
    const selected = analysis?.segments.find((segment) => segment.id === selectedId) ?? null
    const selectedAdvice = ai?.output.segments.find((segment) => segment.segment_id === selectedId) ?? null

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {coach.isPending && <PanelSkeleton label={t('loading')} />}
            {coach.isError && <LoadErrorState title={t('loadError')} isRetrying={coach.isFetching} onRetry={() => void coach.refetch()} />}
            {view && (
                <>
                    <section aria-label={t('playLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p className='text-sm font-medium break-keep'>{view.play.chart.title || view.play.chart.sha256}</p>
                        <p className='text-muted-foreground'>
                            <LampLabel lamp={view.play.lamp} assistGrade={view.play.assist_grade} /> · EX {formatCount(view.play.ex_score)}/
                            {formatCount(view.play.max_ex_score)} · BP {formatCount(view.play.minbp)} · {view.play.random} ·{' '}
                            {formatDateTime(view.play.played_at)}
                            {analysis && ` · ${AI_RUN_END_LABEL[analysis.run_end]}`}
                        </p>
                    </section>
                    {analysis && (
                        <>
                            <AiSegmentChart
                                segments={analysis.segments.map((segment) => ({
                                    id: segment.id,
                                    bp: segment.bp,
                                    gauge: segment.gauge,
                                    isProblem: analysis.problem_segments.includes(segment.id),
                                }))}
                                gaugeMax={GAUGE_MAX}
                                selectedId={selectedId}
                                onSelect={setSelectedId}
                            />
                            <p className='text-muted-foreground text-xs'>{t('chartHint')}</p>
                            {selected && (
                                <section
                                    aria-label={t('selectedLabel')}
                                    className='border-border flex flex-col gap-1 border p-2 text-xs tabular-nums'>
                                    <p className='font-medium'>
                                        {selected.id} ({formatSeconds(selected.from_ms)}~{formatSeconds(selected.to_ms)})
                                    </p>
                                    <p>
                                        {t('selectedStats', {
                                            bp: formatCount(selected.bp),
                                            comboBreaks: formatCount(selected.combo_breaks),
                                            fast: formatCount(selected.fast),
                                            slow: formatCount(selected.slow),
                                            exRatio:
                                                selected.ex_ratio === null
                                                    ? MISSING
                                                    : `${(selected.ex_ratio * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`,
                                        })}
                                    </p>
                                    {selected.top_tags.length > 0 && (
                                        <p className='text-muted-foreground'>
                                            {selected.top_tags.map((tag) => aiPatternTagLabel(tag.tag)).join(' · ')}
                                        </p>
                                    )}
                                    {selectedAdvice && (
                                        <p className='text-sm break-keep'>
                                            {selectedAdvice.diagnosis} {selectedAdvice.advice}
                                        </p>
                                    )}
                                </section>
                            )}
                            {analysis.drops.length > 0 && (
                                <p className='text-xs tabular-nums'>
                                    {t('drops', {
                                        drops: analysis.drops
                                            .map((drop) => `${formatSeconds(drop.from_ms)}~${formatSeconds(drop.to_ms)} −${drop.drop}`)
                                            .join(' · '),
                                    })}
                                </p>
                            )}
                            <ul aria-label={t('otherGaugesLabel')} className='text-muted-foreground flex flex-wrap gap-3 text-xs'>
                                {analysis.gauge.curves.map((curve) => (
                                    <li key={curve.gauge}>
                                        {AI_GAUGE_LABEL[curve.gauge]} {AI_GAUGE_OUTCOME_LABEL[curve.outcome]}
                                    </li>
                                ))}
                            </ul>
                        </>
                    )}
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && ai.output.segments.length > 0 && (
                            <ul aria-label={t('diagnosisLabel')} className='flex flex-col gap-1 text-xs break-keep'>
                                {ai.output.segments.map((segment) => (
                                    <li key={segment.segment_id}>
                                        <button
                                            type='button'
                                            className='font-medium underline-offset-2 hover:underline'
                                            onClick={() => setSelectedId(segment.segment_id)}>
                                            {segment.segment_id}
                                        </button>
                                        : {segment.diagnosis} {segment.advice}
                                    </li>
                                ))}
                            </ul>
                        )}
                        {ai && ai.output.next_try && <p className='text-xs break-keep'>{t('nextTry', { text: ai.output.next_try })}</p>}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
