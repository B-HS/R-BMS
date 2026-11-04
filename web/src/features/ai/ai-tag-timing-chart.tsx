import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import { PERCENT_SCALE } from '@shared/constants/ui'
import type { AiEvidenceGrade } from '@entities/ai.type'

type AiTagTimingRow = { key: string; label: string; deviationMs: number; sdMs: number; fastShare: number; evidence?: AiEvidenceGrade }

const HALF = 50
const MIN_SCALE_MS = 5
const MS_DECIMALS = 1

const signedMs = (value: number) => `${value > 0 ? '+' : ''}${value.toFixed(MS_DECIMALS)} ms`

/**
 * Timing by pattern: each row's mean error against the player's own mean as a bar from the middle (SLOW to the left, FAST to the
 * right) with its spread as a whisker. CSS only, no animation.
 */
export const AiTagTimingChart: FC<{ rows: readonly AiTagTimingRow[]; label: string }> = ({ rows, label }) => {
    const t = useTranslations('pages.ai')
    const scale = Math.max(MIN_SCALE_MS, ...rows.map((row) => Math.abs(row.deviationMs) + row.sdMs))
    const percentOf = (ms: number) => (Math.min(Math.abs(ms), scale) / scale) * HALF

    return (
        <div className='flex flex-col gap-1'>
            <p aria-hidden className='text-muted-foreground grid grid-cols-[8rem_1fr_auto] gap-2 text-xs'>
                <span />
                <span className='flex justify-between'>
                    <span style={{ color: 'var(--rbms-timing-slow)' }}>SLOW</span>
                    <span style={{ color: 'var(--rbms-timing-fast)' }}>FAST</span>
                </span>
                <span />
            </p>
            <ul aria-label={label} className='flex flex-col gap-1 text-xs tabular-nums'>
                {rows.map((row) => {
                    const width = percentOf(row.deviationMs)
                    const whiskerFrom = HALF + Math.sign(row.deviationMs - row.sdMs) * percentOf(row.deviationMs - row.sdMs)
                    const whiskerTo = HALF + Math.sign(row.deviationMs + row.sdMs) * percentOf(row.deviationMs + row.sdMs)
                    return (
                        <li key={row.key} className='grid grid-cols-[8rem_1fr_auto] items-center gap-2'>
                            <span className='truncate'>{row.label}</span>
                            <span className='bg-muted relative h-3'>
                                <span aria-hidden className='bg-border absolute inset-y-0 left-1/2 w-px' />
                                <span
                                    aria-hidden
                                    className='bg-muted-foreground absolute top-1/2 h-px'
                                    style={{ left: `${whiskerFrom}%`, width: `${whiskerTo - whiskerFrom}%` }}
                                />
                                <span
                                    aria-hidden
                                    className='absolute top-0.5 bottom-0.5'
                                    style={
                                        row.deviationMs < 0
                                            ? { right: `${HALF}%`, width: `${width}%`, background: 'var(--rbms-timing-slow)' }
                                            : { left: `${HALF}%`, width: `${width}%`, background: 'var(--rbms-timing-fast)' }
                                    }
                                />
                            </span>
                            <span className='flex items-center gap-1'>
                                {t('profile.timingRow', {
                                    deviation: signedMs(row.deviationMs),
                                    sd: row.sdMs.toFixed(MS_DECIMALS),
                                    fast: Math.round(row.fastShare * PERCENT_SCALE),
                                })}
                                {row.evidence && <Badge variant='outline'>{t(`evidence.${row.evidence}`)}</Badge>}
                            </span>
                        </li>
                    )
                })}
            </ul>
        </div>
    )
}
