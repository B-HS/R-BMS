'use client'

import type { FC } from 'react'
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

const ESTIMATE_LABEL = '추정'

const formatGreen = (value: { green: number; lane_ms: number; is_estimated?: boolean } | null) =>
    value ? `GREEN ${value.green} (${value.lane_ms} ms)${value.is_estimated ? ` ${ESTIMATE_LABEL}` : ''}` : MISSING

export const AiJudgeHabitPanel: FC = () => {
    const habit = useAiJudgeHabit()
    const interpret = useInterpretAiJudgeHabit()

    const view = habit.data
    const ai = interpret.data ?? view?.ai ?? null

    return (
        <PanelCard title='판정 습관 진단' contentClassName='flex flex-col gap-4'>
            {habit.isPending && <PanelSkeleton label='판정 기록 계산 중' />}
            {habit.isError && (
                <LoadErrorState title='판정 기록을 불러오지 못했습니다' isRetrying={habit.isFetching} onRetry={() => void habit.refetch()} />
            )}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>
                        최근 {formatCount(view.plays)}판의 점수 기록으로 계산했습니다(노트 단위 판별 데이터가 생기면 레인별 치우침이 더해집니다).
                    </p>
                    <section aria-label='FAST·SLOW' className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p className='text-sm'>
                            FAST {formatCount(view.fast_slow.fast)} · SLOW {formatCount(view.fast_slow.slow)} · FAST 비율{' '}
                            {view.fast_slow.fast_share === null ? MISSING : `${Math.round(view.fast_slow.fast_share * PERCENT_SCALE)}%`}{' '}
                            <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.fast_slow.grade]}</Badge>
                        </p>
                        <ul className='text-muted-foreground flex flex-wrap gap-3'>
                            {view.fast_slow.by_grade.map((row) => (
                                <li key={row.grade}>
                                    {GRADE_LABEL[row.grade]} 빠름 {formatCount(row.early)} / 늦음 {formatCount(row.late)}
                                </li>
                            ))}
                        </ul>
                    </section>
                    <section aria-label='판정 오프셋' className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p className='text-sm'>
                            평균 치우침 {view.offset.mean_offset_ms === null ? MISSING : `${view.offset.mean_offset_ms} ms`}(양수 = 빠름) ·{' '}
                            <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.offset.grade]}</Badge>
                        </p>
                        <p>
                            제안: {AI_OFFSET_DIRECTION_LABEL[view.offset.direction]}
                            {view.offset.suggested_shift_ms !== null &&
                                ` (JUDGE OFFSET ${view.offset.suggested_shift_ms > 0 ? '+' : ''}${view.offset.suggested_shift_ms} ms)`}
                            . JUDGE OFFSET 은 노트가 보이는 위치를 옮기고 판정은 옮기지 않습니다.
                        </p>
                        <p className='text-muted-foreground'>
                            규칙 세대 4 이상 판 {formatCount(view.offset.plays)}판 · AUTO ADJUST 판 {formatCount(view.offset.auto_adjust_plays)}판은
                            따로 둡니다.
                        </p>
                    </section>
                    <section aria-label='평균 판정 거리' className='text-xs tabular-nums'>
                        평균 판정 거리(avgjudge): 앞쪽 {view.avgjudge.older_ms ?? MISSING} ms → 최근 {view.avgjudge.newer_ms ?? MISSING} ms{' '}
                        <Badge variant='outline'>{AI_EVIDENCE_LABEL[view.avgjudge.grade]}</Badge>
                    </section>
                    <section aria-label='녹색 숫자' className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p>
                            최근 {formatGreen(view.green.latest)} · 가운데값 {formatGreen(view.green.median)} · CONSTANT 판{' '}
                            {formatCount(view.green.constant_plays)}판
                        </p>
                        {view.green.buckets.length > 0 && (
                            <ul aria-label='녹색 숫자 칸' className='text-muted-foreground flex flex-wrap gap-3'>
                                {view.green.buckets.map((bucket) => (
                                    <li key={bucket.key}>
                                        {formatGreen(bucket.from)}~: {formatCount(bucket.plays)}판, FAST{' '}
                                        {bucket.fast_share === null ? MISSING : `${Math.round(bucket.fast_share * PERCENT_SCALE)}%`}
                                    </li>
                                ))}
                            </ul>
                        )}
                    </section>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label='AI 판정 습관 해석'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && (
                            <p className='text-xs break-keep'>
                                오프셋({AI_OFFSET_DIRECTION_LABEL[ai.output.offset_direction]}): {ai.output.offset_reasoning}
                            </p>
                        )}
                        {ai && (
                            <p className='text-xs break-keep'>
                                녹색 숫자({AI_GREEN_DIRECTION_LABEL[ai.output.green_direction]}): {ai.output.green_reasoning}
                            </p>
                        )}
                        {ai && ai.output.notes.length > 0 && <AiInsightList label='판정 메모' insights={ai.output.notes} />}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
