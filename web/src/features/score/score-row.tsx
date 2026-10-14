import type { FC } from 'react'
import Link from 'next/link'
import { LampStrip } from '@features/lamp/lamp-strip'
import { CLEAR_LAMP_LABEL, type ClearLamp } from '@shared/constants/clear-lamp'
import { formatCount, formatDateTime } from '@shared/lib/format'

type ScoreRowProps = {
    href: string
    title: string
    subtitle?: string
    subtitleHref?: string
    lamp: ClearLamp
    exScore: number
    minBp: number
    playedAt: number
}

export const ScoreRow: FC<ScoreRowProps> = ({ href, title, subtitle, subtitleHref, lamp, exScore, minBp, playedAt }) => (
    <li className='border-border hover:bg-muted/50 relative flex min-h-10 items-stretch border-b'>
        <LampStrip lamp={lamp} />
        <div className='flex min-w-0 flex-1 items-center justify-between gap-3 px-3 py-1.5'>
            <div className='flex min-w-0 flex-col'>
                <Link href={href} className='truncate text-xs font-medium outline-none after:absolute after:inset-0 focus-visible:underline'>
                    {title}
                </Link>
                {subtitle && subtitleHref && (
                    <Link
                        href={subtitleHref}
                        className='text-muted-foreground text-2xs relative z-(--z-rail) w-fit max-w-full truncate hover:underline'>
                        {subtitle}
                    </Link>
                )}
                {subtitle && !subtitleHref && <span className='text-muted-foreground text-2xs truncate'>{subtitle}</span>}
            </div>
            <div className='flex shrink-0 flex-col items-end'>
                <span className='text-xs'>
                    {CLEAR_LAMP_LABEL[lamp]} <span className='font-medium tabular-nums'>{formatCount(exScore)}</span>
                </span>
                <span className='text-muted-foreground text-2xs tabular-nums'>
                    BP {formatCount(minBp)} · {formatDateTime(playedAt)}
                </span>
            </div>
        </div>
    </li>
)
