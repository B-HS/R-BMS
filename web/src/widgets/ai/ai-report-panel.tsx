'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
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
    const t = useTranslations('ai.report')
    const report = useAiReport()
    const interpret = useInterpretAiReport()

    const view = report.data
    const ai = interpret.data ?? view?.ai ?? null
    const staleNote = view && view.plays.stale > 0 ? t('staleNote', { stale: view.plays.stale }) : ''
    const failedNote = view && view.plays.failed > 0 ? t('failedNote', { failed: view.plays.failed }) : ''

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {report.isPending && <PanelSkeleton label={t('loading')} />}
            {report.isError && <LoadErrorState title={t('loadError')} isRetrying={report.isFetching} onRetry={() => void report.refetch()} />}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>
                        {t('intro', { analyzed: view.plays.analyzed, measured: view.plays.measured, notes: `${staleNote}${failedNote}` })}
                    </p>
                    {view.tags.length > 0 && <AiTagResidualList label={t('tagsLabel')} rows={view.tags} />}
                    {view.lanes.length > 0 && (
                        <ul aria-label={t('lanesLabel')} className='grid grid-cols-4 gap-1 text-xs tabular-nums sm:grid-cols-8'>
                            {view.lanes.map((lane) => (
                                <li key={lane.key} className='border-border flex flex-col border p-1'>
                                    <span className='font-medium'>{lane.name}</span>
                                    <span>EX {formatRatio(lane.ex_ratio)}</span>
                                    <span className='text-muted-foreground'>FAST {formatRatio(lane.fast_share)}</span>
                                </li>
                            ))}
                        </ul>
                    )}
                    <section aria-label={t('habitsLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                        <p>
                            {t('timing', {
                                mean: view.timing.mean_ms === null ? MISSING : `${view.timing.mean_ms} ms`,
                                fatigue: formatRatio(view.fatigue.mean_delta),
                            })}
                        </p>
                        {view.density.length > 0 && (
                            <p className='text-muted-foreground'>
                                {t('density', {
                                    rows: view.density
                                        .map((row) => t('densityRow', { name: row.name, ratio: formatRatio(row.ex_ratio) }))
                                        .join(' · '),
                                })}
                            </p>
                        )}
                        {view.options.length > 0 && (
                            <p className='text-muted-foreground'>
                                {t('options', {
                                    rows: view.options.map((row) => t('optionRow', { option: row.option, plays: row.plays })).join(' · '),
                                })}
                            </p>
                        )}
                        {view.preferred.length > 0 && (
                            <p className='text-muted-foreground'>
                                {t('preferred', {
                                    rows: view.preferred.map((row) => `${aiPatternTagLabel(row.tag)} +${formatRatio(row.diff)}`).join(' · '),
                                })}
                            </p>
                        )}
                        <p className='text-muted-foreground'>
                            {t('scoreRows', {
                                plays: view.score_rows.plays,
                                fast: formatCount(view.score_rows.fast),
                                slow: formatCount(view.score_rows.slow),
                            })}
                        </p>
                    </section>
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({})}>
                        {ai && <p className='text-sm break-keep'>{ai.output.headline}</p>}
                        {ai && ai.output.strengths.length > 0 && <AiInsightList label={t('strengths')} insights={ai.output.strengths} />}
                        {ai && ai.output.weaknesses.length > 0 && <AiInsightList label={t('weaknesses')} insights={ai.output.weaknesses} />}
                        {ai && ai.output.habits.length > 0 && <AiInsightList label={t('habits')} insights={ai.output.habits} />}
                        {ai && ai.output.practice_focus.length > 0 && (
                            <ul aria-label={t('practiceLabel')} className='list-disc pl-5 text-xs break-keep'>
                                {ai.output.practice_focus.map((step) => (
                                    <li key={`${step.key}-${step.action}`}>{step.action}</li>
                                ))}
                            </ul>
                        )}
                        {ai && ai.output.caveats.length > 0 && (
                            <ul aria-label={t('caveatsLabel')} className='text-muted-foreground list-disc pl-5 text-xs break-keep'>
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
