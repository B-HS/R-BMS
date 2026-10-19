'use client'

import type { FC } from 'react'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiGaugeAdvice, useInterpretAiGaugeAdvice } from '@entities/ai.query'
import { AI_GAUGE_LABEL, AI_OPTION_LABEL, aiPatternTagLabel } from '@entities/ai.type'

const MISSING = '—'
const RATIO_DECIMALS = 1

/** Feature 8: which other gauge would have cleared the failed plays and how each lane option goes, then the AI's advice on a press. */
export const AiGaugeAdvicePanel: FC = () => {
    const advice = useAiGaugeAdvice()
    const interpret = useInterpretAiGaugeAdvice()

    const view = advice.data
    const ai = interpret.data ?? view?.ai ?? null

    return (
        <PanelCard title='게이지·옵션 조언' contentClassName='flex flex-col gap-4'>
            {advice.isPending && <PanelSkeleton label='게이지 기록 모으는 중' />}
            {advice.isError && (
                <LoadErrorState title='게이지 기록을 불러오지 못했습니다' isRetrying={advice.isFetching} onRetry={() => void advice.refetch()} />
            )}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>
                        판별 데이터가 있는 최근 {formatCount(view.plays.analyzed)}판. 게임이 아홉 게이지를 함께 기록하므로 &quot;다른
                        게이지였다면&quot;을 서버가 다시 계산하지 않고 읽습니다. 게이지가 비어 끝난 {formatCount(view.plays.unknown_runs)}판은 그 뒤를
                        알 수 없어 세지 않습니다.
                    </p>
                    {view.gauges.length > 0 && (
                        <ul aria-label='친 게이지' className='flex flex-wrap gap-3 text-xs tabular-nums'>
                            {view.gauges.map((row) => (
                                <li key={row.key}>
                                    {AI_GAUGE_LABEL[row.gauge]} {formatCount(row.cleared)}/{formatCount(row.plays)} 통과
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.rescues.length > 0 && (
                        <ul aria-label='다른 게이지였다면' className='flex flex-col gap-1 text-xs tabular-nums'>
                            {view.rescues.map((row) => (
                                <li key={row.key}>
                                    {row.level} · {AI_GAUGE_LABEL[row.played]} 실패 {formatCount(row.failed)}판
                                    {row.rescued_by.length > 0 &&
                                        ` → ${row.rescued_by.map((rescue) => `${AI_GAUGE_LABEL[rescue.gauge]} ${formatCount(rescue.plays)}판 통과`).join(', ')}`}
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.options.length > 0 && (
                        <ul aria-label='옵션별' className='flex flex-col gap-1 text-xs tabular-nums'>
                            {view.options.map((row) => (
                                <li key={row.key}>
                                    {AI_OPTION_LABEL[row.option]} {formatCount(row.plays)}판 · EX{' '}
                                    {row.ex_ratio === null ? MISSING : `${(row.ex_ratio * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`}
                                    {row.tags.length > 0 &&
                                        ` · ${row.tags.map((tag) => `${aiPatternTagLabel(tag.tag)} ${(tag.residual * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%p`).join(', ')}`}
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.score_rows.length > 0 && (
                        <p className='text-muted-foreground text-xs tabular-nums'>
                            점수 기록:{' '}
                            {view.score_rows
                                .map((row) => `${AI_GAUGE_LABEL[row.gauge]} ${formatCount(row.cleared)}/${formatCount(row.plays)}`)
                                .join(' · ')}
                        </p>
                    )}
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label='AI 게이지·옵션 조언'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {ai && (
                            <ul aria-label='게이지 조언' className='flex flex-col gap-1 text-xs break-keep'>
                                {ai.output.gauge_advice.map((item) => (
                                    <li key={item.gauge}>
                                        <span className='font-medium'>{AI_GAUGE_LABEL[item.gauge]}</span>: {item.reasoning}
                                    </li>
                                ))}
                                {ai.output.option_advice.map((item) => (
                                    <li key={item.option}>
                                        <span className='font-medium'>{AI_OPTION_LABEL[item.option]}</span>: {item.reasoning}
                                    </li>
                                ))}
                            </ul>
                        )}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
