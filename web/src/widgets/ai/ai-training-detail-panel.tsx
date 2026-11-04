'use client'

import { useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { Trash2 } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { AiTrainingBeforeAfter } from '@features/ai/ai-training-before-after'
import { AiTrainingCorrectionList } from '@features/ai/ai-training-correction-list'
import { AiTrainingPlayList } from '@features/ai/ai-training-play-list'
import { AiTrainingSectionStrip } from '@features/ai/ai-training-section-strip'
import { AiTrainingSectionTrend } from '@features/ai/ai-training-section-trend'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { AiTrainingRemovalDialog } from '@widgets/ai/ai-training-removal-dialog'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { ApiError } from '@shared/lib/fetch'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { Button } from '@shared/ui/button'
import { CopyButton } from '@shared/ui/copy-button'
import { useAiTrainingDetail, useAiTrainingFocusLabel } from '@entities/ai.query'
import { AI_TRAINING_PLAYS_SHOWN, aiHubHref, isAiTrainingPositionalKey } from '@entities/ai.type'

const NOT_FOUND_STATUS = 404
const MS_PER_SECOND = 1000
const EX_DECIMALS = 1
const TRAINING_TAB = 'training'

/**
 * One own training chart (plan 2026-11-04 7.3): what it is and how to get it, its sections, what the server changed in the AI's plan,
 * how each section's EX moved over the plays, the plays, and the focus keys before and after. Deleting asks first.
 */
export const AiTrainingDetailPanel: FC<{ chartId: string }> = ({ chartId }) => {
    const returnFocusRef = useRef<HTMLElement | null>(null)
    const [isRemoveOpen, setIsRemoveOpen] = useState(false)

    const router = useRouter()
    const t = useTranslations('pages.ai.training.detail')
    const tTraining = useTranslations('pages.ai.training')
    const detail = useAiTrainingDetail(chartId)
    const focusLabel = useAiTrainingFocusLabel()

    const chart = detail.data
    const isNotFound = detail.error instanceof ApiError && detail.error.status === NOT_FOUND_STATUS
    const hasPositionalSection = chart?.sections.some((section) => isAiTrainingPositionalKey(section.focus_key)) ?? false

    return (
        <>
            {detail.isPending && (
                <PanelCard title={t('title')}>
                    <PanelSkeleton label={t('loading')} />
                </PanelCard>
            )}
            {detail.isError && isNotFound && (
                <PanelCard title={t('title')}>
                    <NotFoundNotice
                        title={t('notFoundTitle')}
                        description={t('notFoundDescription')}
                        actionHref={aiHubHref(TRAINING_TAB)}
                        actionLabel={t('backToList')}
                    />
                </PanelCard>
            )}
            {detail.isError && !isNotFound && (
                <PanelCard title={t('title')}>
                    <LoadErrorState title={t('loadError')} isRetrying={detail.isFetching} onRetry={() => void detail.refetch()} />
                </PanelCard>
            )}
            {chart && (
                <>
                    <PanelCard
                        title={chart.title}
                        contentClassName='flex flex-col gap-3'
                        action={
                            <span className='flex items-center gap-1'>
                                <Button asChild variant='ghost' size='xs'>
                                    <Link href={aiHubHref(TRAINING_TAB)}>{t('backToList')}</Link>
                                </Button>
                                <CopyButton
                                    value={chart.download_url}
                                    label={t('copyUrl')}
                                    copiedMessage={t('copied')}
                                    showLabel
                                    size='xs'
                                    variant='outline'
                                />
                                <Button
                                    type='button'
                                    variant='outline'
                                    size='xs'
                                    onClick={(event) => {
                                        returnFocusRef.current = event.currentTarget
                                        setIsRemoveOpen(true)
                                    }}>
                                    <Trash2 aria-hidden />
                                    {t('remove')}
                                </Button>
                            </span>
                        }>
                        <dl className='grid grid-cols-2 gap-x-4 gap-y-1 text-xs tabular-nums sm:grid-cols-4'>
                            <div>
                                <dt className='text-muted-foreground'>{t('made')}</dt>
                                <dd>{formatDateTime(chart.created_at)}</dd>
                            </div>
                            <div>
                                <dt className='text-muted-foreground'>{t('spec')}</dt>
                                <dd>
                                    {t('specValue', {
                                        bpm: chart.bpm,
                                        seconds: Math.round(chart.length_ms / MS_PER_SECOND),
                                        notes: formatCount(chart.note_count),
                                    })}
                                </dd>
                            </div>
                            <div>
                                <dt className='text-muted-foreground'>{t('playLevel')}</dt>
                                <dd>{chart.play_level}</dd>
                            </div>
                            <div>
                                <dt className='text-muted-foreground'>{t('playsLabel')}</dt>
                                <dd>
                                    {t('playsValue', { count: chart.play_count })}
                                    {chart.best_ex_ratio !== null &&
                                        ` · ${t('best', { ratio: `${(chart.best_ex_ratio * PERCENT_SCALE).toFixed(EX_DECIMALS)}%` })}`}
                                </dd>
                            </div>
                        </dl>
                        <p className='text-muted-foreground text-2xs break-keep'>{t('playLevelNote')}</p>
                        <p className='text-xs break-keep'>
                            <span className='text-muted-foreground'>{t('focus')} </span>
                            {chart.focus_keys.length > 0 ? chart.focus_keys.map(focusLabel).join(', ') : t('noFocus')}
                        </p>
                        {chart.description && <p className='text-xs break-keep'>{chart.description}</p>}
                        {chart.advice && (
                            <p className='text-xs break-keep'>
                                <span className='text-muted-foreground'>{t('advice')} </span>
                                {chart.advice}
                            </p>
                        )}
                        <p className='text-muted-foreground text-xs break-keep'>{t('getInGame')}</p>
                    </PanelCard>
                    <PanelCard title={t('sectionsTitle')} contentClassName='flex flex-col gap-3'>
                        {hasPositionalSection && <p className='text-xs break-keep'>{tTraining('optionNotice')}</p>}
                        <AiTrainingSectionStrip sections={chart.sections} />
                        {chart.corrections.length > 0 && (
                            <section aria-label={t('correctionsTitle')} className='flex flex-col gap-1'>
                                <h3 className='text-sm font-medium'>{t('correctionsTitle')}</h3>
                                <p className='text-muted-foreground text-xs break-keep'>{t('correctionsHint')}</p>
                                <AiTrainingCorrectionList corrections={chart.corrections} />
                            </section>
                        )}
                    </PanelCard>
                    <PanelCard title={t('trendTitle')} contentClassName='flex flex-col gap-3'>
                        <p className='text-muted-foreground text-xs break-keep'>{t('trendHint')}</p>
                        <AiTrainingSectionTrend sections={chart.sections} plays={chart.plays} />
                    </PanelCard>
                    <PanelCard title={t('beforeAfterTitle')} contentClassName='flex flex-col gap-3'>
                        {chart.before_after.length === 0 ? (
                            <p className='text-muted-foreground text-xs break-keep'>{t('beforeAfterEmpty')}</p>
                        ) : (
                            <AiTrainingBeforeAfter items={chart.before_after} />
                        )}
                    </PanelCard>
                    <PanelCard title={t('playsTitle')} contentClassName='flex flex-col gap-3'>
                        {chart.plays.length === 0 ? (
                            <p className='text-muted-foreground text-xs break-keep'>{t('playsEmpty')}</p>
                        ) : (
                            <>
                                <p className='text-muted-foreground text-xs'>
                                    {t('playsShown', { shown: chart.plays.length, total: chart.play_count, max: AI_TRAINING_PLAYS_SHOWN })}
                                </p>
                                <AiTrainingPlayList plays={chart.plays} />
                            </>
                        )}
                    </PanelCard>
                    <AiTrainingRemovalDialog
                        chart={isRemoveOpen ? chart : null}
                        onClose={() => setIsRemoveOpen(false)}
                        onRemoved={() => router.replace(aiHubHref(TRAINING_TAB))}
                        returnFocusRef={returnFocusRef}
                    />
                </>
            )}
        </>
    )
}
