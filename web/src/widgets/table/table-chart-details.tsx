'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { Trophy } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { TableChartRecord } from '@features/table/table-chart-record'
import { TableExternalLink } from '@features/table/table-external-link'
import { AddToFolderMenu } from '@widgets/folder/add-to-folder-menu'
import { useChartLeaderboard } from '@entities/chart.query'
import type { TableSnapshotEntry, TableSnapshotRecord } from '@entities/table-snapshot.type'
import { DEFAULT_INCLUDE_SCRATCH_AUTO } from '@shared/constants/scratch-auto'
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
    const t = useTranslations('browse.table.chartDetails')
    const {
        data: leaderboard,
        isPending: isChartPending,
        isError: isChartError,
    } = useChartLeaderboard(entry.md5, { include_scratch_auto: DEFAULT_INCLUDE_SCRATCH_AUTO })
    const bodyUrl = toWebUrl(entry.url)
    const diffUrl = toWebUrl(entry.url_diff)
    const isSameLink = bodyUrl !== null && bodyUrl === diffUrl
    const diffLabel = entry.name_diff && !WEB_URL_PATTERN.test(entry.name_diff) ? t('addOnNamed', { name: entry.name_diff }) : t('addOn')
    const serverChart = leaderboard?.chart ?? null

    return (
        <>
            <DialogHeader className='border-border gap-1.5 border-b px-4 pt-4 pb-3 pr-12'>
                <Badge variant='secondary' className='text-2xs rounded-none tabular-nums'>
                    {symbol}
                    {entry.level}
                </Badge>
                <DialogTitle className='text-base leading-snug font-semibold wrap-anywhere'>{entry.title || t('untitled')}</DialogTitle>
                <DialogDescription className='text-xs wrap-anywhere'>{entry.artist || t('noArtist')}</DialogDescription>
            </DialogHeader>
            <ScrollContainer variant='always' className='flex flex-col gap-4 p-4'>
                <section aria-label={t('sources')} className='flex flex-col gap-2'>
                    <div className='flex flex-wrap gap-1'>
                        {bodyUrl && <TableExternalLink href={bodyUrl} label={isSameLink ? t('mainAndAddOn') : t('main')} appearance='button' />}
                        {diffUrl && !isSameLink && <TableExternalLink href={diffUrl} label={diffLabel} appearance='button' />}
                        {!bodyUrl && !diffUrl && <p className='text-muted-foreground text-xs'>{t('noSources')}</p>}
                    </div>
                    {(bodyUrl || diffUrl) && <p className='text-muted-foreground text-2xs'>{t('sourcesNote')}</p>}
                </section>
                <section aria-label={t('bestRecord')} className='flex flex-col gap-1.5'>
                    <h3 className='rbms-micro-label'>{t('bestRecord')}</h3>
                    <TableChartRecord record={record} isSignedIn={isSignedIn} isPending={isRecordPending} loginHref={loginHref} />
                </section>
                <dl className='flex min-w-0 items-center gap-2 text-xs'>
                    <dt className='rbms-micro-label'>md5</dt>
                    <dd className='min-w-0 flex-1 truncate font-mono'>{entry.md5}</dd>
                    <CopyButton value={entry.md5} label={t('copyMd5')} variant='ghost' size='icon-xs' />
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
                            {t('chartRanking')}
                            {leaderboard && leaderboard.ranking.length > 0 && (
                                <span className='text-muted-foreground tabular-nums'>{t('playerCount', { count: leaderboard.ranking.length })}</span>
                            )}
                        </Link>
                    </Button>
                )}
                {!isChartPending && isChartError && <span className='text-muted-foreground text-xs'>{t('serverCheckFailed')}</span>}
                {!isChartPending && !isChartError && !serverChart && <span className='text-muted-foreground text-xs'>{t('noServerRecord')}</span>}
            </div>
        </>
    )
}
