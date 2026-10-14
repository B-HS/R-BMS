import type { FC } from 'react'
import { LampStrip } from '@features/lamp/lamp-strip'
import type { TableSnapshotEntry, TableSnapshotRecord } from '@entities/table-snapshot.type'
import { CLEAR_LAMP_LABEL } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'
import { getDjRank, getDjRankClassName } from '@shared/lib/dj-rank'
import { formatCount, getMaxExScore } from '@shared/lib/format'
import { Skeleton } from '@shared/ui/skeleton'

type TableChartCardProps = {
    entry: TableSnapshotEntry
    symbol: string
    record: TableSnapshotRecord | undefined
    isRecordPending: boolean
    onOpen: (entry: TableSnapshotEntry, trigger: HTMLButtonElement) => void
}

export const TableChartCard: FC<TableChartCardProps> = ({ entry, symbol, record, isRecordPending, onOpen }) => {
    const lamp = record?.clear ?? 'NoPlay'
    const maxExScore = record ? getMaxExScore(record.total_notes) : 0
    const rank = record ? getDjRank(record.ex_score, maxExScore) : null
    const title = entry.title || '제목 없음'

    return (
        <button
            type='button'
            aria-haspopup='dialog'
            aria-label={[title, entry.artist, `${symbol}${entry.level}`, CLEAR_LAMP_LABEL[lamp], rank].filter(Boolean).join(' · ')}
            onClick={(event) => onOpen(entry, event.currentTarget)}
            className='bg-card hover:bg-muted/50 focus-visible:ring-ring flex h-14 w-full min-w-0 text-left outline-none focus-visible:ring-2 focus-visible:ring-inset'>
            <LampStrip lamp={isRecordPending ? 'NoPlay' : lamp} />
            <span className='flex min-w-0 flex-1 flex-col justify-between gap-0.5 py-1.5 pr-1.5 pl-2'>
                <span className={cn('text-2xs line-clamp-2 leading-tight font-medium wrap-anywhere', !entry.title && 'text-muted-foreground')}>
                    {title}
                </span>
                <span className='text-2xs text-muted-foreground flex min-w-0 items-center gap-1 leading-tight'>
                    <span className='min-w-0 flex-1 truncate'>{entry.artist}</span>
                    {isRecordPending && <Skeleton className='h-3 w-10 shrink-0' />}
                    {!isRecordPending && record && (
                        <span className='flex shrink-0 items-baseline gap-1 tabular-nums'>
                            <span>{formatCount(record.ex_score)}</span>
                            <span className={getDjRankClassName(record.ex_score, maxExScore)}>{rank}</span>
                        </span>
                    )}
                </span>
            </span>
        </button>
    )
}
