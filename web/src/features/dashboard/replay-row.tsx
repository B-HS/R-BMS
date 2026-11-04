import type { FC } from 'react'
import Link from 'next/link'
import { LampStrip } from '@features/lamp/lamp-strip'
import { lampLabel, type ClearLamp } from '@shared/constants/clear-lamp'
import { formatCount, formatDateTime } from '@shared/lib/format'

type ReplayRowProps = {
    replayId: string
    title: string
    createdAt: number
    lamp: ClearLamp | null
    exScore: number | null
}

export const ReplayRow: FC<ReplayRowProps> = ({ replayId, title, createdAt, lamp, exScore }) => (
    <li className='border-border hover:bg-muted/50 relative flex min-h-10 items-stretch border-b'>
        {lamp ? <LampStrip lamp={lamp} /> : <span aria-hidden='true' className='w-1.25 shrink-0' />}
        <div className='flex min-w-0 flex-1 items-center justify-between gap-3 px-3 py-1.5'>
            <div className='flex min-w-0 flex-col'>
                <Link
                    href={`/replays/${encodeURIComponent(replayId)}`}
                    className='truncate text-xs font-medium outline-none after:absolute after:inset-0 focus-visible:underline'>
                    {title}
                </Link>
                <span className='text-muted-foreground text-2xs tabular-nums'>{formatDateTime(createdAt)}</span>
            </div>
            {lamp && exScore !== null && (
                <span className='shrink-0 text-xs'>
                    {lampLabel(lamp)} <span className='font-medium tabular-nums'>{formatCount(exScore)}</span>
                </span>
            )}
        </div>
    </li>
)
