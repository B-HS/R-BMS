'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { FilterBar } from '@features/filter/filter-bar'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { Pager } from '@features/table/pager'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { SortSelect } from '@features/table/sort-select'
import { usePlayerLeaderboard } from '@entities/stats.query'
import type { PlayerLeaderboardRow } from '@entities/stats.type'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'
import { serializeSearchParams } from '@shared/lib/search-params'
import { PLAYER_SORT_OPTIONS } from '@shared/constants/leaderboard'

const PODIUM_SIZE = 3
const SKELETON_ROWS = 10

const buildColumns = (rankOffset: number): DataTableColumn<PlayerLeaderboardRow>[] => [
    {
        key: 'rank',
        label: '순위',
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
        label: '플레이어',
        flex: true,
        cell: (row) => (
            <Link href={`/players/${encodeURIComponent(row.id)}`} className='font-medium hover:underline'>
                {row.name}
            </Link>
        ),
    },
    { key: 'id', label: 'ID', width: 160, mono: true, hideBelow: 'sm', cell: (row) => row.id },
    { key: 'cleared_charts', label: '클리어', width: 96, align: 'right', cell: (row) => formatCount(row.cleared_charts) },
    { key: 'played_charts', label: '플레이 곡', width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.played_charts) },
    { key: 'total_plays', label: '제출 수', width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.total_plays) },
]

type PlayerLeaderboardPanelProps = {
    params: { sort: string; page: number; limit: number }
}

export const PlayerLeaderboardPanel: FC<PlayerLeaderboardPanelProps> = ({ params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const { data, isLoading, isError, isFetching, refetch } = usePlayerLeaderboard(params)

    const rows = data?.data ?? []
    const totalPages = data?.pagination?.totalPages ?? 1
    const columns = buildColumns((params.page - 1) * params.limit)

    const push = (next: Record<string, string | number | undefined>) => {
        const merged = serializeSearchParams({ sort: params.sort, page: params.page, ...next })
        router.push(merged ? `${pathname}?${merged}` : pathname)
    }

    return (
        <>
            <FilterBar label='정렬' summary={isLoading ? '불러오는 중' : `플레이어 ${formatCount(data?.pagination?.total ?? rows.length)}명`}>
                <SortSelect label='정렬' value={params.sort} options={PLAYER_SORT_OPTIONS} onValueChange={(sort) => push({ sort, page: 1 })} />
            </FilterBar>
            {isLoading && (
                <div className='bg-card p-3'>
                    <TableSkeleton rows={SKELETON_ROWS} label='순위 불러오는 중' />
                </div>
            )}
            {!isLoading && isError && <LoadErrorState title='순위를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && rows.length === 0 && <EmptyState title='아직 순위에 오른 플레이어가 없습니다' />}
            {!isLoading && !isError && rows.length > 0 && (
                <div className='bg-card flex flex-col px-1 pb-2'>
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption='플레이어 종합 순위' />
                    <div className='px-2'>
                        <Pager page={params.page} totalPages={totalPages} onPageChange={(page) => push({ page })} />
                    </div>
                </div>
            )}
        </>
    )
}
