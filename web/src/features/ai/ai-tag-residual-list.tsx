import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import { cn } from '@shared/lib/cn'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { isAiPatternTag, type AiEvidenceGrade } from '@entities/ai.type'

type AiTagResidualRow = { tag: string; residual: number; notes: number; grade: AiEvidenceGrade }

const RESIDUAL_FULL_SCALE = 0.2
const HALF = 50
const RESIDUAL_DECIMALS = 1

/** Tag residuals as bars around zero (left loses EX on that pattern, right gains), with the evidence grade of each. */
export const AiTagResidualList: FC<{ rows: readonly AiTagResidualRow[]; label: string }> = ({ rows, label }) => {
    const t = useTranslations('pages.ai')

    return (
        <ul aria-label={label} className='flex flex-col gap-1 text-xs tabular-nums'>
            {rows.map((row) => {
                const width = (Math.min(Math.abs(row.residual), RESIDUAL_FULL_SCALE) / RESIDUAL_FULL_SCALE) * HALF
                return (
                    <li key={row.tag} className='grid grid-cols-[8rem_1fr_auto] items-center gap-2'>
                        <span className='truncate'>{isAiPatternTag(row.tag) ? t(`patternTag.${row.tag}`) : row.tag}</span>
                        <span className='bg-muted relative h-2'>
                            <span
                                className={cn('absolute top-0 h-2', row.residual < 0 ? 'bg-destructive/70' : 'bg-primary/70')}
                                style={row.residual < 0 ? { right: `${HALF}%`, width: `${width}%` } : { left: `${HALF}%`, width: `${width}%` }}
                            />
                        </span>
                        <span className='flex items-center gap-1'>
                            {row.residual > 0 ? '+' : ''}
                            {(row.residual * PERCENT_SCALE).toFixed(RESIDUAL_DECIMALS)}%p
                            <Badge variant='outline'>{t(`evidence.${row.grade}`)}</Badge>
                        </span>
                    </li>
                )
            })}
        </ul>
    )
}
