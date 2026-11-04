'use client'

import { useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { Trash2 } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { AiTrainingRemovalDialog } from '@widgets/ai/ai-training-removal-dialog'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { Button } from '@shared/ui/button'
import { CopyButton } from '@shared/ui/copy-button'
import { useAiTrainingFocusLabel } from '@entities/ai.query'
import { aiTrainingHref, type AiTrainingChart } from '@entities/ai.type'

const MS_PER_SECOND = 1000
const EX_DECIMALS = 1

/**
 * The own training charts, newest first (plan 2026-11-04 7.3): title and focus, when it was made, how many times it was played and the
 * best EX, the download address to copy, and a delete that opens the dialog that counts what goes and what stays.
 */
export const AiTrainingList: FC<{ charts: readonly AiTrainingChart[] }> = ({ charts }) => {
    const returnFocusRef = useRef<HTMLElement | null>(null)
    const [removeTarget, setRemoveTarget] = useState<AiTrainingChart | null>(null)

    const t = useTranslations('pages.ai.training.list')
    const focusLabel = useAiTrainingFocusLabel()

    return (
        <>
            <ul aria-label={t('label')} className='flex flex-col gap-px'>
                {charts.map((chart) => (
                    <li key={chart.id} className='border-border flex flex-wrap items-center gap-x-3 gap-y-1 border px-2 py-1.5 text-xs tabular-nums'>
                        <div className='flex min-w-48 flex-1 flex-col gap-0.5'>
                            <Link href={aiTrainingHref(chart.id)} className='text-sm font-medium break-keep hover:underline'>
                                {chart.title}
                            </Link>
                            <span className='text-muted-foreground break-keep'>
                                {chart.focus_keys.length > 0 ? chart.focus_keys.map(focusLabel).join(', ') : t('noFocus')}
                            </span>
                            <span className='text-muted-foreground'>
                                {t('made', { date: formatDateTime(chart.created_at) })} ·{' '}
                                {t('spec', {
                                    bpm: chart.bpm,
                                    seconds: Math.round(chart.length_ms / MS_PER_SECOND),
                                    notes: formatCount(chart.note_count),
                                })}
                            </span>
                        </div>
                        <span className='flex flex-col'>
                            <span>{t('plays', { count: chart.plays })}</span>
                            <span className='text-muted-foreground'>
                                {chart.best_ex_ratio === null
                                    ? t('noBest')
                                    : t('best', { ratio: `${(chart.best_ex_ratio * PERCENT_SCALE).toFixed(EX_DECIMALS)}%` })}
                            </span>
                        </span>
                        <span className='ml-auto flex items-center gap-1'>
                            <CopyButton
                                value={chart.download_url}
                                label={t('copyUrl', { title: chart.title })}
                                copiedMessage={t('copied')}
                                size='icon-xs'
                                variant='ghost'
                            />
                            <Button
                                type='button'
                                variant='ghost'
                                size='icon-xs'
                                aria-label={t('remove', { title: chart.title })}
                                onClick={(event) => {
                                    returnFocusRef.current = event.currentTarget
                                    setRemoveTarget(chart)
                                }}>
                                <Trash2 aria-hidden />
                            </Button>
                        </span>
                    </li>
                ))}
            </ul>
            <AiTrainingRemovalDialog chart={removeTarget} onClose={() => setRemoveTarget(null)} returnFocusRef={returnFocusRef} />
        </>
    )
}
