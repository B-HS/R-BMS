import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { LampLabel } from '@features/lamp/lamp-label'
import { CLEAR_LAMPS } from '@shared/constants/clear-lamp'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { Button } from '@shared/ui/button'
import { AI_TRAINING_RANDOM_OFF, aiPlayHref, type AiTrainingPlay } from '@entities/ai.type'

const EX_DECIMALS = 1

/**
 * The plays of one training chart, newest first (plan 2026-11-04 7.2): lamp, EX ratio, BP and the lane option of each, and a link to the
 * coach page of the play. A play without a readable trace has no per-section result.
 */
export const AiTrainingPlayList: FC<{ plays: readonly AiTrainingPlay[] }> = ({ plays }) => {
    const t = useTranslations('pages.ai.training.plays')

    return (
        <ul aria-label={t('label')} className='flex flex-col gap-px'>
            {plays.map((play) => (
                <li key={play.score_id} className='border-border flex flex-wrap items-center gap-x-3 gap-y-1 border px-2 py-1.5 text-xs tabular-nums'>
                    <span>{formatDateTime(play.played_at)}</span>
                    <LampLabel lamp={CLEAR_LAMPS[play.clear] ?? CLEAR_LAMPS[0]} />
                    <span>
                        {play.ex_ratio === null
                            ? t('noEx')
                            : t('ex', { ratio: `${(play.ex_ratio * PERCENT_SCALE).toFixed(EX_DECIMALS)}%`, score: formatCount(play.ex_score) })}
                    </span>
                    <span>BP {formatCount(play.bp)}</span>
                    <span className={play.random === AI_TRAINING_RANDOM_OFF ? 'text-muted-foreground' : undefined}>{play.random.toUpperCase()}</span>
                    {play.sections === null && <span className='text-muted-foreground'>{t('noSections')}</span>}
                    <Button asChild variant='ghost' size='xs' className='ml-auto'>
                        <Link href={aiPlayHref(play.score_id)}>{t('coach')}</Link>
                    </Button>
                </li>
            ))}
        </ul>
    )
}
