'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { DanBadgeList } from '@features/dan/dan-badge-list'
import { FilterBar } from '@features/filter/filter-bar'
import { ScratchAutoToggle } from '@features/filter/scratch-auto-toggle'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { Pager } from '@features/table/pager'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { SortSelect } from '@features/table/sort-select'
import { usePlayerLeaderboard } from '@entities/stats.query'
import type { PlayerLeaderboardParams, PlayerLeaderboardRow } from '@entities/stats.type'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'
import { writeIncludeScratchAutoParam } from '@shared/lib/scratch-auto-param'
import { serializeSearchParams } from '@shared/lib/search-params'
import { INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'

const PODIUM_SIZE = 3
const SKELETON_ROWS = 10
const SORT_VALUES = ['cleared', 'played', 'plays'] as const

type LeaderboardTranslator = ReturnType<typeof useTranslations<'browse.leaderboard'>>

const buildColumns = (rankOffset: number, t: LeaderboardTranslator): DataTableColumn<PlayerLeaderboardRow>[] => [
    {
        key: 'rank',
        label: t('columns.rank'),
        width: 64,
        align: 'right',
        cell: (_row, index) => (
            <span className={cn(rankOffset + index < PODIUM_SIZE ? 'text-foreground font-semibold' : 'text-muted-foreground')}>
                {rankOffset + index + 1}
            </span>
        ),
    },
    {
        key: 'name',
        label: t('columns.player'),
        flex: true,
        cell: (row) => (
            <div className='flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1'>
                <Link href={`/players/${encodeURIComponent(row.id)}`} className='font-medium hover:underline'>
                    {row.name}
                </Link>
                <DanBadgeList dans={row.dans} />
            </div>
        ),
    },
    { key: 'id', label: 'ID', width: 160, mono: true, hideBelow: 'sm', cell: (row) => row.id },
    { key: 'cleared_charts', label: t('columns.cleared'), width: 96, align: 'right', cell: (row) => formatCount(row.cleared_charts) },
    {
        key: 'played_charts',
        label: t('columns.playedCharts'),
        width: 96,
        align: 'right',
        hideBelow: 'sm',
        cell: (row) => formatCount(row.played_charts),
    },
    { key: 'total_plays', label: t('columns.totalPlays'), width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.total_plays) },
]

type PlayerLeaderboardPanelProps = {
    params: PlayerLeaderboardParams
}

export const PlayerLeaderboardPanel: FC<PlayerLeaderboardPanelProps> = ({ params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const t = useTranslations('browse.leaderboard')
    const { data, isLoading, isError, isFetching, refetch } = usePlayerLeaderboard(params)

    const rows = data?.data ?? []
    const totalPages = data?.pagination?.totalPages ?? 1
    const columns = buildColumns((params.page - 1) * params.limit, t)
    const sortOptions = SORT_VALUES.map((value) => ({ value, label: t(`sort.${value}`) }))

    const push = (next: Record<string, string | number | undefined>) => {
        const merged = serializeSearchParams({
            sort: params.sort,
            page: params.page,
            [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(params.include_scratch_auto),
            ...next,
        })
        router.push(merged ? `${pathname}?${merged}` : pathname)
    }

    return (
        <>
            <FilterBar
                label={t('sortLabel')}
                summary={isLoading ? t('summaryLoading') : t('summaryPlayers', { count: data?.pagination?.total ?? rows.length })}>
                <SortSelect label={t('sortLabel')} value={params.sort} options={sortOptions} onValueChange={(sort) => push({ sort, page: 1 })} />
                <ScratchAutoToggle
                    checked={params.include_scratch_auto}
                    onCheckedChange={(checked) => push({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(checked), page: 1 })}
                />
            </FilterBar>
            {isLoading && (
                <div className='bg-card p-3'>
                    <TableSkeleton rows={SKELETON_ROWS} label={t('loading')} />
                </div>
            )}
            {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && rows.length === 0 && <EmptyState title={t('empty')} />}
            {!isLoading && !isError && rows.length > 0 && (
                <div className='bg-card flex flex-col px-1 pb-2'>
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption={t('caption')} />
                    <div className='px-2'>
                        <Pager page={params.page} totalPages={totalPages} onPageChange={(page) => push({ page })} />
                    </div>
                </div>
            )}
            <div className='bg-card px-3 py-2'>
                <PanelFootnote>{t('note')}</PanelFootnote>
            </div>
        </>
    )
}
