'use client'

import type { FC, ReactNode } from 'react'
import { useTranslations } from 'next-intl'
import { AiBpContextList } from '@features/ai/ai-bp-context-list'
import { AiComboGrid } from '@features/ai/ai-combo-grid'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiDensityLimitChart } from '@features/ai/ai-density-limit-chart'
import { AiHandBalance } from '@features/ai/ai-hand-balance'
import { AiSampleBadge } from '@features/ai/ai-sample-badge'
import { AiTagResidualList } from '@features/ai/ai-tag-residual-list'
import { AiTagTimingChart } from '@features/ai/ai-tag-timing-chart'
import { AiWeaknessList } from '@features/ai/ai-weakness-list'
import { PanelCard } from '@features/shell/panel-card'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Badge } from '@shared/ui/badge'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiProfile, useAiWeaknessLabel } from '@entities/ai.query'
import { AI_HAND_KEYS, AI_LANE_NAMES, aiHubHref, formatAiWeaknessValue, splitAiWeaknessKey, type AiWeaknessProfileCell } from '@entities/ai.type'

const TAG_ROWS_SHOWN = 12
const TIMING_ROWS_SHOWN = 10
const CONTEXT_ROWS_SHOWN = 5
const MISSING = '—'
const RATIO_DECIMALS = 1

const kindOf = (cell: AiWeaknessProfileCell) => splitAiWeaknessKey(cell.key)?.kind ?? null
const argumentOf = (cell: AiWeaknessProfileCell) => splitAiWeaknessKey(cell.key)?.parts[0] ?? ''
const isTested = (cell: AiWeaknessProfileCell) => cell.evidence !== 'insufficient' && cell.value !== null

const renderSection = (title: string, children: ReactNode) => (
    <section aria-label={title} className='flex flex-col gap-2'>
        <h3 className='text-sm font-medium'>{title}</h3>
        {children}
    </section>
)

/**
 * The weakness profile (plan 2026-11-04 2.5, 2.7): the patterns a player cannot hit and the density they cannot hold, from the
 * analysed plays of the last 90 days. Every number comes from the server; no AI is called. Each weakness links to a practice folder
 * or a training chart on that key.
 */
export const AiProfilePanel: FC = () => {
    const t = useTranslations('pages.ai.profile')
    const tAi = useTranslations('pages.ai')
    const profile = useAiProfile()
    const weaknessLabel = useAiWeaknessLabel()

    const view = profile.data
    const cells = view?.cells ?? []
    const cellsOf = (kind: string) => cells.filter((cell) => kindOf(cell) === kind)
    const tagRows = cellsOf('tag')
        .filter(isTested)
        .toSorted((left, right) => (left.value ?? 0) - (right.value ?? 0))
        .slice(0, TAG_ROWS_SHOWN)
        .map((cell) => ({ tag: argumentOf(cell), residual: cell.value ?? 0, notes: cell.notes, grade: cell.evidence }))
    const timingRows = cellsOf('timing')
        .filter((cell) => isTested(cell) && cell.timing !== null)
        .toSorted((left, right) => Math.abs(right.value ?? 0) - Math.abs(left.value ?? 0))
        .slice(0, TIMING_ROWS_SHOWN)
        .map((cell) => ({
            key: cell.key,
            label: weaknessLabel(`tag:${argumentOf(cell)}`),
            deviationMs: cell.value ?? 0,
            sdMs: cell.timing?.sd_ms ?? 0,
            fastShare: cell.timing?.fast_share ?? 0,
            evidence: cell.evidence,
        }))
    const handOrder = (cell: AiWeaknessProfileCell) => AI_HAND_KEYS.findIndex((hand) => hand === argumentOf(cell))
    const laneOrder = (cell: AiWeaknessProfileCell) => AI_LANE_NAMES.findIndex((lane) => lane === argumentOf(cell))
    const contextsOf = (kind: string) =>
        cellsOf(kind)
            .toSorted((left, right) => (right.value ?? 0) - (left.value ?? 0))
            .slice(0, CONTEXT_ROWS_SHOWN)
    const npsText = (value: number | null) => (value === null ? t('none') : t('npsValue', { nps: value }))
    const fatigue = cells.find((cell) => cell.key === 'fatigue')

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-5'>
            {profile.isPending && <PanelSkeleton label={t('loading')} />}
            {profile.isError && <LoadErrorState title={t('loadError')} isRetrying={profile.isFetching} onRetry={() => void profile.refetch()} />}
            {view && view.window.plays === 0 && <EmptyState title={t('emptyTitle')} description={t('emptyDescription')} />}
            {view && view.window.plays > 0 && (
                <>
                    <div className='text-muted-foreground flex flex-col gap-1 text-xs break-keep'>
                        <p>
                            {t('intro', {
                                days: view.window.days,
                                plays: formatCount(view.window.plays),
                                charts: formatCount(view.window.charts),
                                notes: formatCount(view.window.notes),
                            })}
                        </p>
                        <p>{t('windowNotes', { auto: view.window.auto_plays, training: view.window.training_plays, stale: view.window.stale })}</p>
                        <p>{t('trainingClusterNote')}</p>
                    </div>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    {renderSection(
                        t('densityTitle'),
                        <>
                            {view.density.curve.length === 0 ? (
                                <p className='text-muted-foreground text-xs'>{t('densityInsufficient')}</p>
                            ) : (
                                <>
                                    <AiDensityLimitChart density={view.density} />
                                    <p className='flex flex-wrap items-center gap-2 text-xs tabular-nums'>
                                        {t('densitySummary', {
                                            comfort: npsText(view.density.comfort_nps),
                                            limitEx: npsText(view.density.limit_nps_ex),
                                            limitBp: npsText(view.density.limit_nps_bp),
                                        })}
                                        <Badge variant='outline'>{tAi(`evidence.${view.density.evidence}`)}</Badge>
                                    </p>
                                    <p className='text-muted-foreground text-xs tabular-nums'>
                                        {t('densitySeen', {
                                            maxSeen: npsText(view.density.max_seen_nps),
                                            peak: npsText(view.density.peak_seen_2s),
                                            baseline:
                                                view.density.baseline_ex === null
                                                    ? MISSING
                                                    : `${(view.density.baseline_ex * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`,
                                        })}
                                    </p>
                                    {!view.density.ceiling_reached_ex && <p className='text-xs break-keep'>{t('noCeiling')}</p>}
                                </>
                            )}
                            <p className='flex flex-wrap items-center gap-2 text-xs tabular-nums'>
                                {t('sustained', {
                                    value: formatAiWeaknessValue('density:sustained', 'ratio', view.density.sustained_minus_burst.value),
                                })}
                                <AiSampleBadge
                                    evidence={view.density.sustained_minus_burst.evidence}
                                    sampleLevel={view.density.sustained_minus_burst.sample_level}
                                />
                            </p>
                        </>,
                    )}
                    {renderSection(
                        t('weaknessesTitle'),
                        <>
                            {view.weaknesses.length === 0 ? (
                                <p className='text-muted-foreground text-xs break-keep'>{t('weaknessesEmpty')}</p>
                            ) : (
                                <AiWeaknessList
                                    label={t('weaknessesTitle')}
                                    items={view.weaknesses}
                                    folderHrefOf={(key) => aiHubHref('recommend', key)}
                                    trainingHrefOf={(key) => aiHubHref('training', key)}
                                />
                            )}
                        </>,
                    )}
                    {view.strengths.length > 0 &&
                        renderSection(
                            t('strengthsTitle'),
                            <>
                                <AiWeaknessList label={t('strengthsTitle')} items={view.strengths} />
                            </>,
                        )}
                    {tagRows.length > 0 &&
                        renderSection(
                            t('tagsTitle'),
                            <>
                                <AiTagResidualList label={t('tagsTitle')} rows={tagRows} />
                            </>,
                        )}
                    {timingRows.length > 0 &&
                        renderSection(
                            t('timingTitle'),
                            <>
                                <AiTagTimingChart label={t('timingTitle')} rows={timingRows} />
                            </>,
                        )}
                    {renderSection(
                        t('comboTitle'),
                        <>
                            <p className='text-muted-foreground text-xs break-keep'>{t('comboHint')}</p>
                            <AiComboGrid cells={cellsOf('combo')} />
                        </>,
                    )}
                    {renderSection(
                        t('handTitle'),
                        <>
                            <AiHandBalance
                                hands={cellsOf('hand').toSorted((left, right) => handOrder(left) - handOrder(right))}
                                lanes={cellsOf('lane').toSorted((left, right) => laneOrder(left) - laneOrder(right))}
                            />
                        </>,
                    )}
                    {renderSection(
                        t('contextTitle'),
                        <>
                            <AiBpContextList
                                bpContexts={contextsOf('bp_context')}
                                gaugeContexts={contextsOf('gauge_context')}
                                bpRuns={view.bp_runs}
                                gauge={view.gauge}
                            />
                        </>,
                    )}
                    <p className='text-muted-foreground text-xs tabular-nums'>
                        {t('soflanLine', {
                            events: formatCount(view.soflan.events),
                            share: view.soflan.adjusted_share === null ? MISSING : `${Math.round(view.soflan.adjusted_share * PERCENT_SCALE)}%`,
                        })}
                        {fatigue && ` · ${t('fatigueLine', { value: formatAiWeaknessValue(fatigue.key, fatigue.unit, fatigue.value) })}`}
                    </p>
                </>
            )}
        </PanelCard>
    )
}
