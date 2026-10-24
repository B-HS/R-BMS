'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
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
    const t = useTranslations('ai.gaugeAdvice')
    const advice = useAiGaugeAdvice()
    const interpret = useInterpretAiGaugeAdvice()

    const view = advice.data
    const ai = interpret.data ?? view?.ai ?? null

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {advice.isPending && <PanelSkeleton label={t('loading')} />}
            {advice.isError && <LoadErrorState title={t('loadError')} isRetrying={advice.isFetching} onRetry={() => void advice.refetch()} />}
            {view && (
                <>
                    <p className='text-muted-foreground text-xs'>{t('intro', { analyzed: view.plays.analyzed, unknown: view.plays.unknown_runs })}</p>
                    {view.gauges.length > 0 && (
                        <ul aria-label={t('playedLabel')} className='flex flex-wrap gap-3 text-xs tabular-nums'>
                            {view.gauges.map((row) => (
                                <li key={row.key}>
                                    {t('playedRow', {
                                        gauge: AI_GAUGE_LABEL[row.gauge],
                                        cleared: formatCount(row.cleared),
                                        plays: formatCount(row.plays),
                                    })}
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.rescues.length > 0 && (
                        <ul aria-label={t('rescuesLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                            {view.rescues.map((row) => (
                                <li key={row.key}>
                                    {t('rescueRow', { level: row.level, gauge: AI_GAUGE_LABEL[row.played], failed: row.failed })}
                                    {row.rescued_by.length > 0 &&
                                        ` → ${row.rescued_by.map((rescue) => t('rescuedBy', { gauge: AI_GAUGE_LABEL[rescue.gauge], plays: rescue.plays })).join(', ')}`}
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.options.length > 0 && (
                        <ul aria-label={t('optionsLabel')} className='flex flex-col gap-1 text-xs tabular-nums'>
                            {view.options.map((row) => (
                                <li key={row.key}>
                                    {t('optionRow', {
                                        option: AI_OPTION_LABEL[row.option],
                                        plays: row.plays,
                                        exRatio: row.ex_ratio === null ? MISSING : `${(row.ex_ratio * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`,
                                    })}
                                    {row.tags.length > 0 &&
                                        ` · ${row.tags.map((tag) => `${aiPatternTagLabel(tag.tag)} ${(tag.residual * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%p`).join(', ')}`}
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.score_rows.length > 0 && (
                        <p className='text-muted-foreground text-xs tabular-nums'>
                            {t('scoreRows', {
                                rows: view.score_rows
                                    .map((row) => `${AI_GAUGE_LABEL[row.gauge]} ${formatCount(row.cleared)}/${formatCount(row.plays)}`)
                                    .join(' · '),
                            })}
                        </p>
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
                        {ai && (
                            <ul aria-label={t('adviceLabel')} className='flex flex-col gap-1 text-xs break-keep'>
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
