'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { ScratchAutoToggle } from '@features/filter/scratch-auto-toggle'
import { ScratchAutoBadge } from '@features/score/scratch-auto-badge'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { StatTile } from '@features/shell/stat-tile'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { LampLabel } from '@features/lamp/lamp-label'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { AddToFolderMenu } from '@widgets/folder/add-to-folder-menu'
import { useChartLeaderboard } from '@entities/chart.query'
import type { ChartLeaderboardParams } from '@entities/chart.type'
import type { ScoreRecord } from '@entities/score.type'
import { displayLampRank, lampLabel } from '@shared/constants/clear-lamp'
import { INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'
import { cn } from '@shared/lib/cn'
import { getDjRank, getDjRankClassName } from '@shared/lib/dj-rank'
import { formatChartTitle, formatCount, formatDateTime, formatHash, getMaxExScore } from '@shared/lib/format'
import { writeIncludeScratchAutoParam } from '@shared/lib/scratch-auto-param'
import { serializeSearchParams } from '@shared/lib/search-params'
import { Badge } from '@shared/ui/badge'
import { CopyButton } from '@shared/ui/copy-button'

const LN_TYPE_LABEL: Record<number, string> = { 0: 'LN', 1: 'CN', 2: 'HCN' }
const META_LABEL_CLASS = 'text-muted-foreground'

const formatBpmRange = (minbpm: number, maxbpm: number) => (minbpm === maxbpm ? String(minbpm) : `${minbpm}–${maxbpm}`)

type ChartTranslator = ReturnType<typeof useTranslations<'browse.chart'>>

const buildColumns = (t: ChartTranslator): DataTableColumn<ScoreRecord>[] => [
    { key: 'rank', label: t('columns.rank'), width: 56, align: 'right', cell: (row) => row.rank ?? '—' },
    {
        key: 'player',
        label: t('columns.player'),
        flex: true,
        cell: (row) => (
            <span className='flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5'>
                <Link href={`/players/${encodeURIComponent(row.player.id)}`} className='max-w-full truncate font-medium hover:underline'>
                    {row.player_name}
                </Link>
                {row.extra.scratch_auto === true && <ScratchAutoBadge />}
            </span>
        ),
    },
    { key: 'clear', label: t('columns.lamp'), width: 144, cell: (row) => <LampLabel lamp={row.clear} assistGrade={row.assist_grade} /> },
    {
        key: 'dj_rank',
        label: t('columns.djRank'),
        width: 64,
        hideBelow: 'sm',
        cell: (row) => (
            <span className={getDjRankClassName(row.ex_score, getMaxExScore(row.total_notes))}>
                {getDjRank(row.ex_score, getMaxExScore(row.total_notes))}
            </span>
        ),
    },
    { key: 'ex_score', label: 'EX', width: 80, align: 'right', cell: (row) => formatCount(row.ex_score) },
    { key: 'minbp', label: 'BP', width: 64, align: 'right', cell: (row) => formatCount(row.minbp) },
    { key: 'max_combo', label: t('columns.combo'), width: 80, align: 'right', hideBelow: 'md', cell: (row) => formatCount(row.max_combo) },
    { key: 'played_at', label: t('columns.playedAt'), width: 144, align: 'right', hideBelow: 'md', cell: (row) => formatDateTime(row.played_at) },
    {
        key: 'replay',
        label: t('columns.replay'),
        width: 80,
        align: 'right',
        cell: (row) =>
            typeof row.extra.replay_id === 'string' ? (
                <Link href={`/replays/${encodeURIComponent(row.extra.replay_id)}`} className='hover:underline'>
                    {t('columns.replayLink')}
                </Link>
            ) : (
                <span className='text-muted-foreground'>—</span>
            ),
    },
]

type ChartTableMembership = {
    tableId: string
    name: string
    symbol: string
    level: string
    href: string
}

type ChartLeaderboardPanelProps = {
    hash: string
    params: ChartLeaderboardParams
    tableMemberships: ChartTableMembership[]
}

export const ChartLeaderboardPanel: FC<ChartLeaderboardPanelProps> = ({ hash, params, tableMemberships }) => {
    const router = useRouter()
    const pathname = usePathname()
    const t = useTranslations('browse.chart')
    const { data, isLoading, isError, isFetching, refetch } = useChartLeaderboard(hash, params)

    const handleScratchAutoChange = (checked: boolean) => {
        const query = serializeSearchParams({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(checked) })
        router.push(query ? `${pathname}?${query}` : pathname, { scroll: false })
    }

    if (isLoading) return <ShellPageLoading title={t('pageTitle')} />

    if (isError)
        return (
            <ShellPage title={t('pageTitle')}>
                <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
            </ShellPage>
        )

    if (!data?.chart)
        return (
            <ShellPage title={t('pageTitle')}>
                <NotFoundNotice
                    title={t('notFoundTitle')}
                    description={t('notFoundDescription')}
                    actionHref='/search'
                    actionLabel={t('notFoundAction')}
                />
            </ShellPage>
        )

    const { chart, ranking } = data
    const columns = buildColumns(t)
    const primaryMembership = tableMemberships.at(0)
    const chartLevel = chart.level === null ? undefined : String(chart.level)
    const folderLevel = primaryMembership ? `${primaryMembership.symbol}${primaryMembership.level}` : chartLevel
    const bestExScore = ranking.reduce((best, row) => Math.max(best, row.ex_score), 0)
    const bestLamp = ranking.reduce<Pick<ScoreRecord, 'clear' | 'assist_grade'>>(
        (best, row) =>
            displayLampRank(row.clear, row.assist_grade) > displayLampRank(best.clear, best.assist_grade)
                ? { clear: row.clear, assist_grade: row.assist_grade }
                : best,
        { clear: 'NoPlay', assist_grade: 0 },
    )

    return (
        <ShellPage
            title={formatChartTitle(chart)}
            description={chart.artist || undefined}
            actions={<AddToFolderMenu md5={chart.md5} sha256={chart.sha256} title={chart.title} artist={chart.artist} level={folderLevel} />}>
            <div className='bg-card flex flex-col gap-2 p-3'>
                <p className={cn('text-lg leading-tight font-semibold tracking-tight wrap-anywhere', !chart.title.trim() && 'text-muted-foreground')}>
                    {formatChartTitle(chart)}
                </p>
                {(chart.subtitle || chart.artist) && (
                    <p className='text-muted-foreground text-xs'>{[chart.subtitle, chart.artist].filter(Boolean).join(' · ')}</p>
                )}
                {tableMemberships.length > 0 && (
                    <ul aria-label={t('membershipsLabel')} className='flex flex-wrap gap-1'>
                        {tableMemberships.map((membership) => (
                            <li key={`${membership.tableId}-${membership.level}`}>
                                <Badge asChild variant='outline' className='h-6 rounded-none text-xs'>
                                    <Link
                                        href={membership.href}
                                        aria-label={t('membershipLink', { name: membership.name, level: `${membership.symbol}${membership.level}` })}>
                                        <span className='font-semibold tabular-nums'>
                                            {membership.symbol}
                                            {membership.level}
                                        </span>
                                        <span className='text-muted-foreground'>{membership.name}</span>
                                    </Link>
                                </Badge>
                            </li>
                        ))}
                    </ul>
                )}
                <dl className='flex flex-wrap gap-x-4 gap-y-1 text-xs'>
                    <div className='flex gap-1.5'>
                        <dt className={META_LABEL_CLASS}>{t('meta.mode')}</dt>
                        <dd>{chart.mode}</dd>
                    </div>
                    <div className='flex gap-1.5'>
                        <dt className={META_LABEL_CLASS}>{t('meta.level')}</dt>
                        <dd className='tabular-nums'>{chart.level ?? '—'}</dd>
                    </div>
                    <div className='flex gap-1.5'>
                        <dt className={META_LABEL_CLASS}>{t('meta.notes')}</dt>
                        <dd className='tabular-nums'>{formatCount(chart.notes)}</dd>
                    </div>
                    <div className='flex gap-1.5'>
                        <dt className={META_LABEL_CLASS}>BPM</dt>
                        <dd className='tabular-nums'>{formatBpmRange(chart.minbpm, chart.maxbpm)}</dd>
                    </div>
                    <div className='flex gap-1.5'>
                        <dt className={META_LABEL_CLASS}>TOTAL</dt>
                        <dd className='tabular-nums'>{chart.total ?? '—'}</dd>
                    </div>
                    <div className='flex gap-1.5'>
                        <dt className={META_LABEL_CLASS}>{t('meta.longNote')}</dt>
                        <dd>{LN_TYPE_LABEL[chart.lntype] ?? 'LN'}</dd>
                    </div>
                </dl>
                <dl className='flex flex-wrap gap-x-4 gap-y-1 text-xs'>
                    <div className='flex items-center gap-1.5'>
                        <dt className={META_LABEL_CLASS}>md5</dt>
                        <dd className='font-mono'>{formatHash(chart.md5)}</dd>
                        <CopyButton value={chart.md5} label={t('copyMd5')} variant='ghost' size='icon-xs' />
                    </div>
                    <div className='flex items-center gap-1.5'>
                        <dt className={META_LABEL_CLASS}>sha256</dt>
                        <dd className='font-mono'>{formatHash(chart.sha256)}</dd>
                        <CopyButton value={chart.sha256} label={t('copySha256')} variant='ghost' size='icon-xs' />
                    </div>
                </dl>
            </div>

            <div className='grid grid-cols-2 gap-px md:grid-cols-4'>
                <StatTile label={t('stats.bestEx')} value={formatCount(bestExScore)} />
                <StatTile label={t('stats.bestLamp')} value={lampLabel(bestLamp.clear, bestLamp.assist_grade)} />
                <StatTile
                    label={t('stats.rankedPlayers')}
                    value={formatCount(new Set(ranking.map((row) => row.player.id)).size)}
                    hint={t('stats.scopeHint')}
                />
                <StatTile label={t('stats.shown')} value={formatCount(ranking.length)} hint={t('stats.scopeHint')} />
            </div>

            <section aria-labelledby='chart-ranking-heading' className='bg-card flex flex-col'>
                <div className='flex min-h-9 flex-wrap items-center justify-between gap-2 px-3 py-1'>
                    <h2 id='chart-ranking-heading' className='text-sm font-semibold'>
                        {t('ranking')}
                    </h2>
                    <ScratchAutoToggle checked={params.include_scratch_auto} onCheckedChange={handleScratchAutoChange} />
                </div>
                {ranking.length === 0 && <EmptyState title={t('emptyTitle')} description={t('emptyDescription')} />}
                {ranking.length > 0 && (
                    <div className='px-1'>
                        <DataTable columns={columns} rows={ranking} rowKey={(row) => `${row.player.id}-${row.played_at}`} caption={t('caption')} />
                    </div>
                )}
                <div className='px-3 py-2'>
                    <PanelFootnote>{t('footnote')}</PanelFootnote>
                </div>
            </section>
        </ShellPage>
    )
}
