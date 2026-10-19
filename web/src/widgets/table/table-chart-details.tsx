'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { Trophy } from 'lucide-react'
import { TableChartRecord } from '@features/table/table-chart-record'
import { TableExternalLink } from '@features/table/table-external-link'
import { AddToFolderMenu } from '@widgets/folder/add-to-folder-menu'
import { useChartLeaderboard } from '@entities/chart.query'
import type { TableSnapshotEntry, TableSnapshotRecord } from '@entities/table-snapshot.type'
import { DEFAULT_INCLUDE_SCRATCH_AUTO } from '@shared/constants/scratch-auto'
import { formatCount } from '@shared/lib/format'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { CopyButton } from '@shared/ui/copy-button'
import { DialogDescription, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { ScrollContainer } from '@shared/ui/scroll-container'
import { Skeleton } from '@shared/ui/skeleton'

type TableChartDetailsProps = {
    entry: TableSnapshotEntry
    symbol: string
    record: TableSnapshotRecord | undefined
    isSignedIn: boolean
    isRecordPending: boolean
    loginHref: string
}

const WEB_URL_PATTERN = /^https?:\/\//i

const toWebUrl = (value: string) => {
    if (!WEB_URL_PATTERN.test(value)) return null
    try {
        return new URL(value).href
    } catch {
        return null
    }
}

export const TableChartDetails: FC<TableChartDetailsProps> = ({ entry, symbol, record, isSignedIn, isRecordPending, loginHref }) => {
    const {
        data: leaderboard,
        isPending: isChartPending,
        isError: isChartError,
    } = useChartLeaderboard(entry.md5, { include_scratch_auto: DEFAULT_INCLUDE_SCRATCH_AUTO })
    const bodyUrl = toWebUrl(entry.url)
    const diffUrl = toWebUrl(entry.url_diff)
    const isSameLink = bodyUrl !== null && bodyUrl === diffUrl
    const diffLabel = entry.name_diff && !WEB_URL_PATTERN.test(entry.name_diff) ? `차분: ${entry.name_diff}` : '차분'
    const serverChart = leaderboard?.chart ?? null

    return (
        <>
            <DialogHeader className='border-border gap-1.5 border-b px-4 pt-4 pb-3 pr-12'>
                <Badge variant='secondary' className='text-2xs rounded-none tabular-nums'>
                    {symbol}
                    {entry.level}
                </Badge>
                <DialogTitle className='text-base leading-snug font-semibold wrap-anywhere'>{entry.title || '제목 없음'}</DialogTitle>
                <DialogDescription className='text-xs wrap-anywhere'>{entry.artist || '아티스트 정보 없음'}</DialogDescription>
            </DialogHeader>
            <ScrollContainer variant='always' className='flex flex-col gap-4 p-4'>
                <section aria-label='받는 곳' className='flex flex-col gap-2'>
                    <div className='flex flex-wrap gap-1'>
                        {bodyUrl && <TableExternalLink href={bodyUrl} label={isSameLink ? '본편 · 차분' : '본편'} appearance='button' />}
                        {diffUrl && !isSameLink && <TableExternalLink href={diffUrl} label={diffLabel} appearance='button' />}
                        {!bodyUrl && !diffUrl && <p className='text-muted-foreground text-xs'>표에 받는 곳이 적혀 있지 않습니다.</p>}
                    </div>
                    {(bodyUrl || diffUrl) && (
                        <p className='text-muted-foreground text-2xs'>원작자의 주소로 잇습니다. 오래된 주소는 열리지 않을 수 있습니다.</p>
                    )}
                </section>
                <section aria-label='내 최고 기록' className='flex flex-col gap-1.5'>
                    <h3 className='rbms-micro-label'>내 최고 기록</h3>
                    <TableChartRecord record={record} isSignedIn={isSignedIn} isPending={isRecordPending} loginHref={loginHref} />
                </section>
                <dl className='flex min-w-0 items-center gap-2 text-xs'>
                    <dt className='rbms-micro-label'>md5</dt>
                    <dd className='min-w-0 flex-1 truncate font-mono'>{entry.md5}</dd>
                    <CopyButton value={entry.md5} label='md5 복사' variant='ghost' size='icon-xs' />
                </dl>
            </ScrollContainer>
            <div className='border-border flex flex-wrap items-center gap-2 border-t px-4 py-3'>
                <AddToFolderMenu
                    md5={entry.md5}
                    sha256={serverChart?.sha256}
                    title={entry.title}
                    artist={entry.artist}
                    level={`${symbol}${entry.level}`}
                />
                {isChartPending && <Skeleton className='h-7 w-28' />}
                {!isChartPending && serverChart && (
                    <Button asChild variant='outline' size='sm'>
                        <Link href={`/charts/${serverChart.sha256}`}>
                            <Trophy aria-hidden />
                            차트 순위
                            {leaderboard && leaderboard.ranking.length > 0 && (
                                <span className='text-muted-foreground tabular-nums'>{formatCount(leaderboard.ranking.length)}명</span>
                            )}
                        </Link>
                    </Button>
                )}
                {!isChartPending && isChartError && <span className='text-muted-foreground text-xs'>서버의 차트 정보를 확인하지 못했습니다.</span>}
                {!isChartPending && !isChartError && !serverChart && (
                    <span className='text-muted-foreground text-xs'>서버에 아직 이 차트의 기록이 없습니다.</span>
                )}
            </div>
        </>
    )
}
