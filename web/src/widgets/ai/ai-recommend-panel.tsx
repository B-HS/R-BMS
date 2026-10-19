'use client'

import type { FC } from 'react'
import { AiChartCandidateList } from '@features/ai/ai-chart-candidate-list'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Badge } from '@shared/ui/badge'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiRecommend, useInterpretAiRecommend } from '@entities/ai.query'
import { AI_RECOMMEND_KIND_LABEL } from '@entities/ai.type'

const SHOWN_CANDIDATES = 10
const NEARBY_LEVELS = 2

export const AiRecommendPanel: FC = () => {
    const recommend = useAiRecommend(null)
    const interpret = useInterpretAiRecommend()

    const view = recommend.data
    const ai = interpret.data ?? view?.ai ?? null
    const byMd5 = new Map((view?.candidates ?? []).map((candidate) => [candidate.md5, candidate]))
    const picks = (ai?.output.picks ?? []).flatMap((pick) => {
        const candidate = byMd5.get(pick.md5)
        return candidate ? [{ candidate, reason: pick.reason }] : []
    })
    const reasons = new Map(picks.map((pick) => [pick.candidate.md5, pick.reason]))
    const frontierIndex = view?.frontier ? view.levels.findIndex((level) => level.level === view.frontier?.level) : -1
    const nearby = frontierIndex < 0 ? [] : (view?.levels.slice(Math.max(0, frontierIndex - NEARBY_LEVELS), frontierIndex + NEARBY_LEVELS + 1) ?? [])

    return (
        <PanelCard title='다음 도전 곡' contentClassName='flex flex-col gap-4'>
            {recommend.isPending && <PanelSkeleton label='추천 후보 계산 중' />}
            {recommend.isError && (
                <LoadErrorState title='추천 후보를 불러오지 못했습니다' isRetrying={recommend.isFetching} onRetry={() => void recommend.refetch()} />
            )}
            {view && (
                <>
                    <section aria-label='경계 레벨' className='flex flex-col gap-2'>
                        {view.frontier && (
                            <p className='text-sm'>
                                지금 도전하는 레벨은{' '}
                                <strong>
                                    {view.table.symbol}
                                    {view.frontier.level}
                                </strong>{' '}
                                입니다. 클리어 {formatCount(view.frontier.cleared)}/{formatCount(view.frontier.charts)}곡.
                            </p>
                        )}
                        <ul aria-label='근처 레벨' className='flex flex-wrap gap-2'>
                            {nearby.map((level) => (
                                <li key={level.key} className='border-border flex flex-col border px-2 py-1 text-xs tabular-nums'>
                                    <span className='font-medium'>
                                        {view.table.symbol}
                                        {level.level}
                                    </span>
                                    <span className='text-muted-foreground'>클리어 {Math.round(level.clear_rate * PERCENT_SCALE)}%</span>
                                </li>
                            ))}
                        </ul>
                        <p className='text-muted-foreground text-xs'>
                            후보는 서버가 레벨·램프로 고릅니다. 곡별 패턴이 있는 후보 {formatCount(view.pattern_coverage.with_features)}/
                            {formatCount(view.pattern_coverage.candidates)}곡
                            {view.pattern_coverage.stale_features > 0 &&
                                ` (그중 옛 분석 버전 ${formatCount(view.pattern_coverage.stale_features)}곡은 다시 계산하기 전의 값)`}
                            . 패턴 태그는 옵션을 끈 원본 배치 기준입니다.
                        </p>
                    </section>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {picks.length === 0 && view.candidates.length > 0 && (
                        <AiChartCandidateList
                            label='추천 후보'
                            symbol={view.table.symbol}
                            candidates={view.candidates.slice(0, SHOWN_CANDIDATES)}
                            renderExtra={(candidate) => {
                                const kind = view.candidates.find((entry) => entry.md5 === candidate.md5)?.kind
                                return kind && <Badge variant='secondary'>{AI_RECOMMEND_KIND_LABEL[kind]}</Badge>
                            }}
                        />
                    )}
                    <AiInterpretSection
                        label='AI 추천 해석'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({ table: view.table.id })}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {picks.length > 0 && (
                            <AiChartCandidateList
                                label='AI 가 고른 순서'
                                symbol={view.table.symbol}
                                candidates={picks.map((pick) => pick.candidate)}
                                renderExtra={(candidate) => <p className='text-foreground text-xs break-keep'>{reasons.get(candidate.md5)}</p>}
                            />
                        )}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
