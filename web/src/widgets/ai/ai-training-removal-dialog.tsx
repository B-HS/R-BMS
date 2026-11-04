'use client'

import type { FC, RefObject } from 'react'
import { useTranslations } from 'next-intl'
import { ConfirmAction } from '@features/dialog/confirm-action'
import { useAiTrainingRemoval, useRemoveAiTrainingChart } from '@entities/ai.query'
import type { AiTrainingChart } from '@entities/ai.type'

type AiTrainingRemovalDialogProps = {
    chart: Pick<AiTrainingChart, 'id' | 'title'> | null
    onClose: () => void
    onRemoved?: () => void
    returnFocusRef?: RefObject<HTMLElement | null>
}

/**
 * Asks before deleting one training chart (plan 2026-11-04 7.3). The server counts what the delete takes away and what it leaves when
 * the dialog opens, the dialog says those counts, and only the confirm button sends the DELETE; it stays off until the counts are in.
 */
export const AiTrainingRemovalDialog: FC<AiTrainingRemovalDialogProps> = ({ chart, onClose, onRemoved, returnFocusRef }) => {
    const t = useTranslations('pages.ai.training.removal')
    const chartId = chart?.id ?? ''
    const removal = useAiTrainingRemoval(chartId, chart !== null)
    const remove = useRemoveAiTrainingChart()
    const counts = removal.data

    return (
        <ConfirmAction
            open={chart !== null}
            onOpenChange={(open) => !open && onClose()}
            title={t('title', { title: chart?.title ?? '' })}
            description={
                <>
                    {removal.isPending && <span className='block'>{t('loading')}</span>}
                    {removal.isError && <span className='text-destructive block'>{t('loadError')}</span>}
                    {counts && (
                        <>
                            <span className='block'>
                                {t('removes', { charts: counts.removes.training_chart, folderItems: counts.removes.folder_items })}
                            </span>
                            <span className='block'>
                                {t('keeps', {
                                    scores: counts.keeps.scores,
                                    traces: counts.keeps.traces,
                                    replays: counts.keeps.replays,
                                    patterns: counts.keeps.chart_pattern,
                                })}
                            </span>
                            <span className='block'>{t('keepsFiles')}</span>
                        </>
                    )}
                </>
            }
            confirmLabel={t('confirm')}
            confirmDisabled={counts === undefined || remove.isPending}
            returnFocusRef={returnFocusRef}
            onConfirm={() => remove.mutate(chartId, { onSuccess: onRemoved })}
        />
    )
}
