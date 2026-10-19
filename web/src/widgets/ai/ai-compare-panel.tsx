'use client'

import { useState, type FC } from 'react'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { AiCompareTotalsTable } from '@features/ai/ai-compare-totals-table'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { formatCount } from '@shared/lib/format'
import { useAiCompare, useInterpretAiCompare } from '@entities/ai.query'
import { AI_COMPARE_REASON_LABEL, aiPatternTagLabel, type AiCompareQuery } from '@entities/ai.type'

const BEST_TARGET = 'best'
const RIVAL_PREFIX = 'rival:'

const queryOf = (target: string): AiCompareQuery =>
    target.startsWith(RIVAL_PREFIX) ? { against: 'rival', rival: target.slice(RIVAL_PREFIX.length) } : { against: 'best' }

const MISSING = '—'

const signed = (value: number | null) => {
    if (value === null) return MISSING
    return `${value > 0 ? '+' : ''}${formatCount(value)}`
}

/**
 * Feature 3: this play against my best other play on the chart or a rival's. Segment differences show only when both plays have an
 * analysis and, for a rival, when that rival shares segment totals; otherwise the public judgement totals are set side by side.
 */
export const AiComparePanel: FC<{ scoreId: string }> = ({ scoreId }) => {
    const [target, setTarget] = useState(BEST_TARGET)

    const query = queryOf(target)
    const compare = useAiCompare(scoreId, query)
    const interpret = useInterpretAiCompare(scoreId)

    const view = compare.data
    const isSwitching = compare.isPlaceholderData
    const ai = interpret.data ?? view?.ai ?? null
    const gapIds = new Set(view?.gap_segments ?? [])
    const unreachedCount = view?.segments.filter((segment) => segment.ex_diff === null).length ?? 0
    const isPartial = view?.mode === 'segments' && unreachedCount > 0
    const reachedExDiff = view?.segments.reduce((sum, segment) => sum + (segment.ex_diff ?? 0), 0) ?? 0

    const handleTargetChange = (next: string) => {
        setTarget(next)
        interpret.reset()
    }

    return (
        <PanelCard
            title='베스트·라이벌 비교'
            action={
                <Select value={target} onValueChange={handleTargetChange}>
                    <SelectTrigger size='sm' aria-label='비교할 판' className='w-48 max-w-full'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={BEST_TARGET}>내 다른 최고 판</SelectItem>
                        {(view?.rivals ?? []).map((rival) => (
                            <SelectItem key={rival.login_id} value={`${RIVAL_PREFIX}${rival.login_id}`}>
                                {rival.name}
                                {rival.allows_segments ? ' (구간 공유)' : ' (합계만)'}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            }
            contentClassName='flex flex-col gap-4'>
            {compare.isPending && <PanelSkeleton label='비교 계산 중' />}
            {compare.isError && (
                <LoadErrorState title='비교를 불러오지 못했습니다' isRetrying={compare.isFetching} onRetry={() => void compare.refetch()} />
            )}
            {isSwitching && <PanelSkeleton label='비교 대상 바꾸는 중' />}
            {view && !isSwitching && (
                <>
                    {view.reason && (
                        <p role='status' className='text-muted-foreground text-xs break-keep'>
                            {AI_COMPARE_REASON_LABEL[view.reason]}
                        </p>
                    )}
                    {view.target && (
                        <p className='text-xs tabular-nums'>
                            이 판 EX {formatCount(view.base.ex_score)} · 비교 판({view.target.who === 'rival' ? view.target.login_id : '나'}) EX{' '}
                            {formatCount(view.target.ex_score)} · {isPartial ? '도달한 구간의 차이' : '차이'}{' '}
                            {signed(isPartial ? reachedExDiff : view.base.ex_score - view.target.ex_score)}
                        </p>
                    )}
                    {isPartial && (
                        <p role='status' className='text-muted-foreground text-xs break-keep'>
                            {`한쪽 판이 도달하지 못한 ${formatCount(unreachedCount)}구간은 비교에서 뺐습니다.`}
                        </p>
                    )}
                    {view.mode === 'segments' && (
                        <ol aria-label='구간별 EX 차' className='flex flex-col gap-0.5 text-xs tabular-nums'>
                            {view.segments
                                .filter((segment) => gapIds.has(segment.id))
                                .map((segment) => (
                                    <li key={segment.key}>
                                        {segment.id}: EX {signed(segment.ex_diff)} · BP {signed(segment.bp_diff)} · 누적{' '}
                                        {signed(segment.cumulative_ex_diff)}
                                        {segment.top_tags.length > 0 && ` · ${segment.top_tags.map((tag) => aiPatternTagLabel(tag.tag)).join(', ')}`}
                                    </li>
                                ))}
                        </ol>
                    )}
                    {view.mode === 'totals' && view.totals && <AiCompareTotalsTable totals={view.totals} />}
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {view.mode === 'segments' && (
                        <AiInterpretSection
                            label='AI 비교 해석'
                            meta={ai}
                            isPending={interpret.isPending}
                            error={interpret.error}
                            isBlocked={view.gap_segments.length === 0}
                            onInterpret={() => interpret.mutate(query)}>
                            {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                            {ai && (
                                <ul aria-label='차이 설명' className='flex flex-col gap-1 text-xs break-keep'>
                                    {ai.output.gaps.map((gap) => (
                                        <li key={gap.segment_id}>
                                            <span className='font-medium'>{gap.segment_id}</span>: {gap.explanation}
                                        </li>
                                    ))}
                                </ul>
                            )}
                            {ai && ai.output.takeaway && <p className='text-xs break-keep'>{ai.output.takeaway}</p>}
                        </AiInterpretSection>
                    )}
                </>
            )}
        </PanelCard>
    )
}
