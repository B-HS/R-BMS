'use client'

import { useState, type FC } from 'react'
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

    const coach = useAiCoach(scoreId)
    const interpret = useInterpretAiCoach(scoreId)

    const view = coach.data
    const analysis = view?.analysis ?? null
    const ai = interpret.data ?? view?.ai ?? null
    const selected = analysis?.segments.find((segment) => segment.id === selectedId) ?? null
    const selectedAdvice = ai?.output.segments.find((segment) => segment.segment_id === selectedId) ?? null

    return (
        <PanelCard title='리플레이 코치' contentClassName='flex flex-col gap-4'>
            {coach.isPending && <PanelSkeleton label='판 분석 불러오는 중' />}
            {coach.isError && (
                <LoadErrorState title='판 분석을 불러오지 못했습니다' isRetrying={coach.isFetching} onRetry={() => void coach.refetch()} />
            )}
            {view && (
                <>
                    <section aria-label='판' className='flex flex-col gap-1 text-xs tabular-nums'>
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
                            <p className='text-muted-foreground text-xs'>막대는 구간별 BP, 선은 친 게이지입니다. 막대를 누르면 그 구간을 봅니다.</p>
                            {selected && (
                                <section aria-label='고른 구간' className='border-border flex flex-col gap-1 border p-2 text-xs tabular-nums'>
                                    <p className='font-medium'>
                                        {selected.id} ({formatSeconds(selected.from_ms)}~{formatSeconds(selected.to_ms)})
                                    </p>
                                    <p>
                                        BP {formatCount(selected.bp)} · 콤보 끊김 {formatCount(selected.combo_breaks)} · FAST{' '}
                                        {formatCount(selected.fast)} / SLOW {formatCount(selected.slow)} · EX{' '}
                                        {selected.ex_ratio === null ? MISSING : `${(selected.ex_ratio * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`}
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
                                    게이지 꺾임:{' '}
                                    {analysis.drops
                                        .map((drop) => `${formatSeconds(drop.from_ms)}~${formatSeconds(drop.to_ms)} −${drop.drop}`)
                                        .join(' · ')}
                                </p>
                            )}
                            <ul aria-label='다른 게이지였다면' className='text-muted-foreground flex flex-wrap gap-3 text-xs'>
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
                        label='AI 코치'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && ai.output.segments.length > 0 && (
                            <ul aria-label='구간 진단' className='flex flex-col gap-1 text-xs break-keep'>
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
                        {ai && ai.output.next_try && <p className='text-xs break-keep'>다음 판: {ai.output.next_try}</p>}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
