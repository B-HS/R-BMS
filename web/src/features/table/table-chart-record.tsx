import type { FC } from 'react'
import Link from 'next/link'
import { LampLabel } from '@features/lamp/lamp-label'
import { ScratchAutoBadge } from '@features/score/scratch-auto-badge'
import type { TableSnapshotRecord } from '@entities/table-snapshot.type'
import { cn } from '@shared/lib/cn'
import { getDjRank, getDjRankClassName } from '@shared/lib/dj-rank'
import { formatCount, formatDateTime, formatPercent, getMaxExScore } from '@shared/lib/format'
import { Button } from '@shared/ui/button'
import { Skeleton } from '@shared/ui/skeleton'

type TableChartRecordProps = {
    record: TableSnapshotRecord | undefined
    isSignedIn: boolean
    isPending: boolean
    loginHref: string
}

const BOX_CLASS_NAME = 'border-border flex flex-col gap-2 border p-2'

export const TableChartRecord: FC<TableChartRecordProps> = ({ record, isSignedIn, isPending, loginHref }) => {
    if (!isSignedIn)
        return (
            <div className={BOX_CLASS_NAME}>
                <p className='text-muted-foreground text-xs'>로그인하면 내 램프와 점수가 표시됩니다.</p>
                <Button asChild variant='outline' size='sm' className='self-start'>
                    <Link href={loginHref}>로그인</Link>
                </Button>
            </div>
        )

    if (isPending)
        return (
            <div className={BOX_CLASS_NAME} role='status' aria-busy='true'>
                <span className='sr-only'>내 기록 불러오는 중</span>
                <Skeleton className='h-4 w-32' />
                <Skeleton className='h-4 w-48' />
            </div>
        )

    if (!record)
        return (
            <div className={BOX_CLASS_NAME}>
                <LampLabel lamp='NoPlay' className='text-xs' />
                <p className='text-muted-foreground text-xs'>아직 이 차트의 기록이 없습니다.</p>
            </div>
        )

    const maxExScore = getMaxExScore(record.total_notes)

    return (
        <div className={BOX_CLASS_NAME}>
            <div className='flex items-center justify-between gap-2'>
                <span className='flex items-center gap-1.5'>
                    <LampLabel lamp={record.clear} className='text-sm font-medium' />
                    {record.scratch_auto && <ScratchAutoBadge />}
                </span>
                <span className={cn('text-sm tabular-nums', getDjRankClassName(record.ex_score, maxExScore))}>
                    {getDjRank(record.ex_score, maxExScore)}
                </span>
            </div>
            <dl className='grid grid-cols-1 gap-x-4 gap-y-1 text-xs sm:grid-cols-2'>
                <div className='flex justify-between gap-2'>
                    <dt className='text-muted-foreground'>EX</dt>
                    <dd className='tabular-nums'>
                        {formatCount(record.ex_score)} <span className='text-muted-foreground'>({formatPercent(record.ex_score, maxExScore)})</span>
                    </dd>
                </div>
                <div className='flex justify-between gap-2'>
                    <dt className='text-muted-foreground'>BP</dt>
                    <dd className='tabular-nums'>{formatCount(record.minbp)}</dd>
                </div>
                <div className='flex justify-between gap-2'>
                    <dt className='text-muted-foreground'>최대 콤보</dt>
                    <dd className='tabular-nums'>
                        {formatCount(record.max_combo)} <span className='text-muted-foreground'>/ {formatCount(record.total_notes)}</span>
                    </dd>
                </div>
                <div className='flex justify-between gap-2'>
                    <dt className='text-muted-foreground'>기록 시각</dt>
                    <dd className='tabular-nums'>{formatDateTime(record.played_at)}</dd>
                </div>
            </dl>
        </div>
    )
}
