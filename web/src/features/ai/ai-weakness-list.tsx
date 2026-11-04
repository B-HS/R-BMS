import type { FC } from 'react'
import Link from 'next/link'
import { TrendingDown, TrendingUp } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { AiSampleBadge } from '@features/ai/ai-sample-badge'
import { Button } from '@shared/ui/button'
import { formatCount } from '@shared/lib/format'
import { useAiWeaknessLabel } from '@entities/ai.query'
import { formatAiWeaknessValue, type AiWeaknessItem } from '@entities/ai.type'

type AiWeaknessListProps = {
    label: string
    items: readonly AiWeaknessItem[]
    folderHrefOf?: (key: string) => string
    trainingHrefOf?: (key: string) => string
}

/**
 * The strongest weaknesses (or strengths) of the profile: name, value, grade with its sample, the change of the last 30 days against
 * the 60 days before, and links that open a practice folder or a training chart on that key when the key can make one.
 */
export const AiWeaknessList: FC<AiWeaknessListProps> = ({ label, items, folderHrefOf, trainingHrefOf }) => {
    const t = useTranslations('pages.ai.profile')
    const weaknessLabel = useAiWeaknessLabel()

    return (
        <ul aria-label={label} className='flex flex-col gap-px'>
            {items.map((item) => (
                <li key={item.key} className='border-border flex flex-wrap items-center gap-x-3 gap-y-1 border px-2 py-1.5 text-xs tabular-nums'>
                    <span className='min-w-32 text-sm font-medium break-keep'>{weaknessLabel(item.key)}</span>
                    <span>{formatAiWeaknessValue(item.key, item.unit, item.value)}</span>
                    <AiSampleBadge evidence={item.evidence} sampleLevel={item.sample_level} />
                    <span className='text-muted-foreground'>
                        {t('itemSample', { notes: formatCount(item.notes), charts: formatCount(item.charts), plays: formatCount(item.plays) })}
                    </span>
                    {item.trend_delta !== null && item.trend_delta !== 0 && (
                        <span className='text-muted-foreground flex items-center gap-1'>
                            {item.trend_delta > 0 ? <TrendingUp aria-hidden className='size-3' /> : <TrendingDown aria-hidden className='size-3' />}
                            {t('trend', { delta: formatAiWeaknessValue(item.key, item.unit, item.trend_delta) })}
                        </span>
                    )}
                    <span className='ml-auto flex items-center gap-1'>
                        {folderHrefOf && item.selectable_for_folder && (
                            <Button asChild variant='outline' size='xs'>
                                <Link href={folderHrefOf(item.key)} aria-label={t('folderLinkLabel', { name: weaknessLabel(item.key) })}>
                                    {t('folderLink')}
                                </Link>
                            </Button>
                        )}
                        {trainingHrefOf && item.training_kinds.length > 0 && (
                            <Button asChild variant='outline' size='xs'>
                                <Link href={trainingHrefOf(item.key)} aria-label={t('trainingLinkLabel', { name: weaknessLabel(item.key) })}>
                                    {t('trainingLink')}
                                </Link>
                            </Button>
                        )}
                    </span>
                </li>
            ))}
        </ul>
    )
}
