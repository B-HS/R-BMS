'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiChartCandidateList } from '@features/ai/ai-chart-candidate-list'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Badge } from '@shared/ui/badge'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useAiRecommend, useInterpretAiRecommend } from '@entities/ai.query'
import { AI_RECOMMEND_KIND_LABEL } from '@entities/ai.type'

const SHOWN_CANDIDATES = 10
const NEARBY_LEVELS = 2

export const AiRecommendPanel: FC = () => {
    const t = useTranslations('ai.recommend')
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
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {recommend.isPending && <PanelSkeleton label={t('loading')} />}
            {recommend.isError && (
                <LoadErrorState title={t('loadError')} isRetrying={recommend.isFetching} onRetry={() => void recommend.refetch()} />
            )}
            {view && (
                <>
                    <section aria-label={t('frontierLabel')} className='flex flex-col gap-2'>
                        {view.frontier && (
                            <p className='text-sm'>
                                {t.rich('frontier', {
                                    level: `${view.table.symbol}${view.frontier.level}`,
                                    cleared: view.frontier.cleared,
                                    charts: view.frontier.charts,
                                    strong: (chunks) => <strong>{chunks}</strong>,
                                })}
                            </p>
                        )}
                        <ul aria-label={t('nearbyLabel')} className='flex flex-wrap gap-2'>
                            {nearby.map((level) => (
                                <li key={level.key} className='border-border flex flex-col border px-2 py-1 text-xs tabular-nums'>
                                    <span className='font-medium'>
                                        {view.table.symbol}
                                        {level.level}
                                    </span>
                                    <span className='text-muted-foreground'>
                                        {t('clearRate', { rate: Math.round(level.clear_rate * PERCENT_SCALE) })}
                                    </span>
                                </li>
                            ))}
                        </ul>
                        <p className='text-muted-foreground text-xs'>
                            {view.pattern_coverage.stale_features > 0
                                ? t('coverageStale', {
                                      withFeatures: view.pattern_coverage.with_features,
                                      candidates: view.pattern_coverage.candidates,
                                      stale: view.pattern_coverage.stale_features,
                                  })
                                : t('coverage', {
                                      withFeatures: view.pattern_coverage.with_features,
                                      candidates: view.pattern_coverage.candidates,
                                  })}
                        </p>
                    </section>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {picks.length === 0 && view.candidates.length > 0 && (
                        <AiChartCandidateList
                            label={t('candidatesLabel')}
                            symbol={view.table.symbol}
                            candidates={view.candidates.slice(0, SHOWN_CANDIDATES)}
                            renderExtra={(candidate) => {
                                const kind = view.candidates.find((entry) => entry.md5 === candidate.md5)?.kind
                                return kind && <Badge variant='secondary'>{AI_RECOMMEND_KIND_LABEL[kind]}</Badge>
                            }}
                        />
                    )}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({ table: view.table.id })}>
                        {ai && <p className='text-sm break-keep'>{ai.output.summary}</p>}
                        {picks.length > 0 && (
                            <AiChartCandidateList
                                label={t('picksLabel')}
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
