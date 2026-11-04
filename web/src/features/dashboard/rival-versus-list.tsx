import type { FC } from 'react'
import Link from 'next/link'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'

type RivalVersusListRow = {
    login_id: string
    name: string
    common: number
    wins: number
    losses: number
    draws: number
}

type RivalVersusListProps = {
    rows: readonly RivalVersusListRow[]
    label: string
    describeRow: (row: RivalVersusListRow) => string
    noCommonLabel: string
}

const percentOf = (part: number, whole: number) => (part / whole) * PERCENT_SCALE

/** One row per rival: the name, and the common charts split into wins, draws and losses of the player's best EX score against the rival's. */
export const RivalVersusList: FC<RivalVersusListProps> = ({ rows, label, describeRow, noCommonLabel }) => (
    <ul aria-label={label} className='flex flex-col gap-1.5'>
        {rows.map((row) => (
            <li key={row.login_id} className='grid grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)] items-center gap-2 text-xs'>
                <Link href={`/players/${encodeURIComponent(row.login_id)}`} className='truncate hover:underline'>
                    {row.name}
                </Link>
                {row.common === 0 ? (
                    <span className='text-muted-foreground text-2xs truncate'>{noCommonLabel}</span>
                ) : (
                    <span role='img' aria-label={describeRow(row)} className='flex min-w-0 items-center gap-1.5'>
                        <span className='bg-muted flex h-2 min-w-0 flex-1 overflow-hidden'>
                            <span className='bg-primary' style={{ width: `${percentOf(row.wins, row.common)}%` }} />
                            <span className='bg-muted-foreground/50' style={{ width: `${percentOf(row.draws, row.common)}%` }} />
                        </span>
                        <span className='text-muted-foreground shrink-0 tabular-nums'>
                            {formatCount(row.wins)}-{formatCount(row.draws)}-{formatCount(row.losses)}
                        </span>
                    </span>
                )}
            </li>
        ))}
    </ul>
)
