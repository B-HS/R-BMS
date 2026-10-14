import type { FC, ReactNode } from 'react'
import Link from 'next/link'
import { ChevronRight } from 'lucide-react'
import { formatUrlHost } from '@features/table/format-url-host'
import { LampStackBar, type LampCount } from '@features/lamp/lamp-stack-bar'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { Skeleton } from '@shared/ui/skeleton'

type TableSnapshotCardProps = {
    table: TableSnapshotSummary
    progress: { lamps: LampCount[]; played: number } | null
    isProgressPending: boolean
    isSignedIn: boolean
    action?: ReactNode
}

export const TableSnapshotCard: FC<TableSnapshotCardProps> = ({ table, progress, isProgressPending, isSignedIn, action }) => {
    const firstLevel = table.levels[0]?.level
    const lastLevel = table.levels[table.levels.length - 1]?.level

    return (
        <article className='bg-card hover:bg-muted/40 relative flex min-w-0 flex-col gap-3 p-3 sm:flex-row sm:items-center'>
            <div className='flex min-w-0 flex-1 items-center gap-3'>
                <span aria-hidden className='bg-muted flex size-12 shrink-0 items-center justify-center text-xl font-semibold'>
                    {table.symbol}
                </span>
                <div className='flex min-w-0 flex-col gap-1'>
                    <h3 className='min-w-0'>
                        <Link
                            href={table.standard_paths.page}
                            className='focus-visible:ring-ring flex items-center gap-1 text-base font-semibold outline-none after:absolute after:inset-0 hover:underline focus-visible:after:ring-2 focus-visible:after:ring-inset'>
                            <span className='truncate'>{table.name}</span>
                            <ChevronRight aria-hidden className='text-muted-foreground size-4 shrink-0' />
                        </Link>
                    </h3>
                    <p className='text-muted-foreground text-xs tabular-nums'>
                        {formatCount(table.entry_count)}곡 ·{' '}
                        {firstLevel && lastLevel && `${table.symbol}${firstLevel} ~ ${table.symbol}${lastLevel} · `}
                        {formatCount(table.level_count)}단계 · 원본 갱신 {table.last_update} · 출처 {formatUrlHost(table.source.page_url)}
                    </p>
                </div>
            </div>
            <div className='flex min-w-0 items-center gap-3 sm:w-72 sm:shrink-0'>
                <div className='flex min-w-0 flex-1 flex-col gap-1'>
                    {progress && (
                        <>
                            <LampStackBar lamps={progress.lamps} />
                            <span className='text-muted-foreground text-xs tabular-nums'>
                                <span className='text-foreground font-medium'>{formatCount(progress.played)}</span> / {formatCount(table.entry_count)}{' '}
                                플레이 · {table.entry_count === 0 ? 0 : Math.round((progress.played / table.entry_count) * PERCENT_SCALE)}%
                            </span>
                        </>
                    )}
                    {!progress && isProgressPending && (
                        <>
                            <Skeleton className='h-3 w-full' />
                            <Skeleton className='h-3 w-24' />
                        </>
                    )}
                    {!progress && !isProgressPending && (
                        <span className='text-muted-foreground text-xs'>
                            {isSignedIn ? '내 기록을 불러오지 못했습니다.' : '로그인하면 내 진행이 보입니다.'}
                        </span>
                    )}
                </div>
                {action && <div className='relative z-(--z-raised) shrink-0'>{action}</div>}
            </div>
        </article>
    )
}
