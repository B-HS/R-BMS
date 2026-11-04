import type { FC } from 'react'
import { TrendingDown, TrendingUp } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import { formatCount } from '@shared/lib/format'
import { useAiWeaknessLabel } from '@entities/ai.query'
import { formatAiWeaknessDelta, formatAiWeaknessValue, isAiEvidenceGrade, type AiTrainingFocusComparison } from '@entities/ai.type'

/**
 * Each focus key of a training chart before the chart was made and over the plays since (plan 2026-11-04 7.2, 7.3): the profile's
 * value when it was planned, the value measured on the plays after it with the same sample conditions, their difference, and the
 * grade of the measurement. A key with too few plays since shows no "after", not a guess.
 */
export const AiTrainingBeforeAfter: FC<{ items: readonly AiTrainingFocusComparison[] }> = ({ items }) => {
    const t = useTranslations('pages.ai.training.beforeAfter')
    const tEvidence = useTranslations('pages.ai.evidence')
    const weaknessLabel = useAiWeaknessLabel()

    return (
        <div className='flex flex-col gap-2'>
            <ul aria-label={t('listLabel')} className='flex flex-col gap-px'>
                {items.map((item) => (
                    <li
                        key={item.focus_key}
                        className='border-border grid grid-cols-1 gap-x-4 gap-y-1 border px-2 py-1.5 text-xs tabular-nums sm:grid-cols-[minmax(8rem,1fr)_1fr_1fr_auto]'>
                        <span className='text-sm font-medium break-keep'>{weaknessLabel(item.focus_key)}</span>
                        <span className='flex flex-col'>
                            <span className='text-muted-foreground'>{t('before')}</span>
                            <span>{formatAiWeaknessValue(item.focus_key, item.unit, item.before.value)}</span>
                            <span className='text-muted-foreground'>
                                {t('sample', { notes: formatCount(item.before.notes), charts: formatCount(item.before.charts) })}
                            </span>
                        </span>
                        <span className='flex flex-col'>
                            <span className='text-muted-foreground'>{t('after')}</span>
                            {item.after ? (
                                <>
                                    <span>{formatAiWeaknessValue(item.focus_key, item.unit, item.after.value)}</span>
                                    <span className='text-muted-foreground'>
                                        {t('afterSample', {
                                            plays: formatCount(item.after.plays),
                                            notes: formatCount(item.after.notes),
                                            charts: formatCount(item.after.charts),
                                        })}
                                    </span>
                                </>
                            ) : (
                                <span className='text-muted-foreground break-keep'>{t('afterMissing')}</span>
                            )}
                        </span>
                        <span className='flex flex-wrap items-center gap-1 sm:flex-col sm:items-end'>
                            {item.delta !== null && (
                                <span className='flex items-center gap-1 font-medium'>
                                    {item.delta > 0 && <TrendingUp aria-hidden className='size-3' />}
                                    {item.delta < 0 && <TrendingDown aria-hidden className='size-3' />}
                                    {t('delta', { delta: formatAiWeaknessDelta(item.focus_key, item.unit, item.delta) })}
                                </span>
                            )}
                            {item.after && isAiEvidenceGrade(item.after.evidence) && (
                                <Badge variant='outline'>{tEvidence(item.after.evidence)}</Badge>
                            )}
                        </span>
                    </li>
                ))}
            </ul>
            <p className='text-muted-foreground text-2xs break-keep'>{t('hint')}</p>
        </div>
    )
}
