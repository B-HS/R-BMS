'use client'

import type { FC } from 'react'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInsightList } from '@features/ai/ai-insight-list'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { AiTagResidualList } from '@features/ai/ai-tag-residual-list'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiReport, useInterpretAiReport } from '@entities/ai.query'
import { aiPatternTagLabel } from '@entities/ai.type'

const MISSING = '—'
const RATIO_DECIMALS = 1

const formatRatio = (ratio: number | null) => (ratio === null ? MISSING : `${(ratio * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`)

/** Feature 1: the deterministic report from the newest analysed plays first, then the AI's reading of it on a button press. */
export const AiReportPanel: FC = () => {
    const report = useAiReport()
    const interpret = useInterpretAiReport()

    const view = report.data
    const ai = interpret.data ?? view?.ai ?? null

    return (
        <PanelCard title='플레이어 리포트' contentClassName='flex flex-col gap-4'>
            {report.isPending && <PanelSkeleton label='판별 분석 모으는 중' />}
            {report.isError && (
                <LoadErrorState title='리포트를 불러오지 못했습니다' isRetrying={report.isFetching} onRetry={() => void report.refetch()} />
            )}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>
                        판별 데이터가 있는 최근 {formatCount(view.plays.analyzed)}판 중 7키 단일 플레이 {formatCount(view.plays.measured)}판으로
                        계산했습니다
                        {view.plays.stale > 0 && `(옛 분석 버전 ${formatCount(view.plays.stale)}판은 다시 계산되면 더해집니다)`}
                        {view.plays.failed > 0 && `(분석하지 못한 ${formatCount(view.plays.failed)}판은 뺐습니다)`}. 레인은 옵션을 적용한 뒤 실제로 친
                        레인이고, 손은 기본 손 배정 기준입니다.
                    </p>
                    {view.tags.length > 0 && <AiTagResidualList label='패턴별 잔차' rows={view.tags} />}
                    {view.lanes.length > 0 && (
                        <ul aria-label='레인별 정확도' className='grid grid-cols-4 gap-1 text-xs tabular-nums sm:grid-cols-8'>
                            {view.lanes.map((lane) => (
                                <li key={lane.key} className='border-border flex flex-col border p-1'>
                                    <span className='font-medium'>{lane.name}</span>
                                    <span>EX {formatRatio(lane.ex_ratio)}</span>
                                    <span className='text-muted-foreground'>FAST {formatRatio(lane.fast_share)}</span>
                                </li>
                            ))}
                        </ul>
                    )}
                    <section aria-label='습관' className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p>
                            오차 치우침 {view.timing.mean_ms === null ? MISSING : `${view.timing.mean_ms} ms`}(양수 = 빠름) · 피로(뒤 1/3 − 앞 1/3 EX){' '}
                            {formatRatio(view.fatigue.mean_delta)}
                        </p>
                        {view.density.length > 0 && (
                            <p className='text-muted-foreground'>
                                밀도별 EX: {view.density.map((row) => `${row.name}개/초 ${formatRatio(row.ex_ratio)}`).join(' · ')}
                            </p>
                        )}
                        {view.options.length > 0 && (
                            <p className='text-muted-foreground'>
                                옵션: {view.options.map((row) => `${row.option} ${formatCount(row.plays)}판`).join(' · ')}
                            </p>
                        )}
                        {view.preferred.length > 0 && (
                            <p className='text-muted-foreground'>
                                자주 고른 곡의 패턴(원본 배치):{' '}
                                {view.preferred.map((row) => `${aiPatternTagLabel(row.tag)} +${formatRatio(row.diff)}`).join(' · ')}
                            </p>
                        )}
                        <p className='text-muted-foreground'>
                            점수 기록 최근 {formatCount(view.score_rows.plays)}판: FAST {formatCount(view.score_rows.fast)} · SLOW{' '}
                            {formatCount(view.score_rows.slow)}
                        </p>
                    </section>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label='AI 리포트'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.headline}</p>}
                        {ai && ai.output.strengths.length > 0 && <AiInsightList label='강점' insights={ai.output.strengths} />}
                        {ai && ai.output.weaknesses.length > 0 && <AiInsightList label='약점' insights={ai.output.weaknesses} />}
                        {ai && ai.output.habits.length > 0 && <AiInsightList label='습관' insights={ai.output.habits} />}
                        {ai && ai.output.practice_focus.length > 0 && (
                            <ul aria-label='연습할 것' className='list-disc pl-5 text-xs break-keep'>
                                {ai.output.practice_focus.map((step) => (
                                    <li key={`${step.key}-${step.action}`}>{step.action}</li>
                                ))}
                            </ul>
                        )}
                        {ai && ai.output.caveats.length > 0 && (
                            <ul aria-label='한계' className='text-muted-foreground list-disc pl-5 text-xs break-keep'>
                                {ai.output.caveats.map((caveat) => (
                                    <li key={caveat}>{caveat}</li>
                                ))}
                            </ul>
                        )}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
