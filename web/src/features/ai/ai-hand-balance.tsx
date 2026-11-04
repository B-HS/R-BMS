import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { cn } from '@shared/lib/cn'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useAiWeaknessLabel } from '@entities/ai.query'
import { formatAiWeaknessValue, type AiWeaknessProfileCell } from '@entities/ai.type'

const RESIDUAL_FULL_SCALE = 0.1
const HALF = 50
const RATIO_DECIMALS = 1
const MS_DECIMALS = 1
const MISSING = '—'

/**
 * The hands and the lanes side by side: each one's EX residual as a bar from the middle (left loses EX against the play's level,
 * right gains), its BP rate and its mean timing. Two CSS bar rows, no animation; lanes are the lanes played after options.
 */
export const AiHandBalance: FC<{ hands: readonly AiWeaknessProfileCell[]; lanes: readonly AiWeaknessProfileCell[] }> = ({ hands, lanes }) => {
    const t = useTranslations('pages.ai.profile')
    const weaknessLabel = useAiWeaknessLabel()

    const renderRow = (cell: AiWeaknessProfileCell) => {
        const residual = cell.value ?? 0
        const width = (Math.min(Math.abs(residual), RESIDUAL_FULL_SCALE) / RESIDUAL_FULL_SCALE) * HALF
        return (
            <li key={cell.key} className='grid grid-cols-[6rem_1fr] items-center gap-x-2 gap-y-0.5 sm:grid-cols-[6rem_8rem_1fr]'>
                <span className='truncate font-medium'>{weaknessLabel(cell.key)}</span>
                <span aria-hidden className='bg-muted relative h-2'>
                    <span className='bg-border absolute inset-y-0 left-1/2 w-px' />
                    <span
                        className={cn('absolute top-0 h-2', residual < 0 ? 'bg-destructive/70' : 'bg-primary/70')}
                        style={residual < 0 ? { right: `${HALF}%`, width: `${width}%` } : { left: `${HALF}%`, width: `${width}%` }}
                    />
                </span>
                <span className='text-muted-foreground col-span-2 sm:col-span-1'>
                    {t('handRow', {
                        residual: formatAiWeaknessValue(cell.key, cell.unit, cell.value),
                        bp: cell.bp_rate === null ? MISSING : `${(cell.bp_rate * PERCENT_SCALE).toFixed(RATIO_DECIMALS)}%`,
                        timing:
                            cell.timing === null ? MISSING : `${cell.timing.mean_ms > 0 ? '+' : ''}${cell.timing.mean_ms.toFixed(MS_DECIMALS)} ms`,
                    })}
                    {cell.evidence === 'insufficient' && ` · ${t('sampleLevel.insufficient')}`}
                </span>
            </li>
        )
    }

    return (
        <div className='grid gap-3 text-xs tabular-nums md:grid-cols-2'>
            <ul aria-label={t('handsLabel')} className='flex flex-col gap-1'>
                {hands.map(renderRow)}
            </ul>
            <ul aria-label={t('lanesLabel')} className='flex flex-col gap-1'>
                {lanes.map(renderRow)}
            </ul>
        </div>
    )
}
