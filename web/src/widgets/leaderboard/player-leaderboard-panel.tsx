'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { PanelCard } from '@features/shell/panel-card'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { Pager } from '@features/table/pager'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { SortSelect } from '@features/table/sort-select'
import { usePlayerLeaderboard } from '@entities/stats.query'
import type { PlayerLeaderboardRow } from '@entities/stats.type'
import { formatCount } from '@shared/lib/format'
import { serializeSearchParams } from '@shared/lib/search-params'

const SORT_OPTIONS = [
    { value: 'rank_points', label: '랭크 포인트' },
    { value: 'total_plays', label: '플레이 수' },
]

const RANK_DECIMALS = 2

const buildColumns = (rankOffset: number): DataTableColumn<PlayerLeaderboardRow>[] => [
    { key: 'rank', label: '순위', width: 64, align: 'right', cell: (_row, index) => rankOffset + index + 1 },
    {
        key: 'name',
        label: '플레이어',
        flex: true,
        cell: (row) => (
            <Link href={`/players/${row.id}`} className='hover:underline'>
                {row.name}
            </Link>
        ),
    },
    { key: 'id', label: 'ID', width: 128, mono: true, cell: (row) => row.id },
    { key: 'rank_points', label: '랭크 포인트', width: 128, align: 'right', cell: (row) => row.rank_points.toFixed(RANK_DECIMALS) },
    { key: 'total_plays', label: '플레이', width: 96, align: 'right', cell: (row) => formatCount(row.total_plays) },
]

type PlayerLeaderboardPanelProps = {
    params: { sort: string; page: number; limit: number }
}

export const PlayerLeaderboardPanel: FC<PlayerLeaderboardPanelProps> = ({ params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const { data, isLoading, isError } = usePlayerLeaderboard(params)

    const rows = data?.data ?? []
    const totalPages = data?.pagination?.totalPages ?? 1
    const columns = buildColumns((params.page - 1) * params.limit)

    const push = (next: Record<string, string | number | undefined>) => {
        const merged = serializeSearchParams({ sort: params.sort, page: params.page, ...next })
        router.push(merged ? `${pathname}?${merged}` : pathname)
    }

    return (
        <>
            <PanelCard title='정렬'>
                <SortSelect label='정렬' value={params.sort} options={SORT_OPTIONS} onValueChange={(sort) => push({ sort, page: 1 })} />
            </PanelCard>
            <PanelCard title={`플레이어 ${formatCount(data?.pagination?.total ?? rows.length)}명`} contentClassName='flex flex-col gap-2'>
                {isLoading && <TableSkeleton />}
                {!isLoading && isError && <LoadErrorState title='리더보드를 불러오지 못했습니다' />}
                {!isLoading && !isError && rows.length === 0 && <EmptyState title='표시할 플레이어가 없습니다' />}
                {!isLoading && !isError && rows.length > 0 && (
                    <>
                        <DataTable columns={columns} rows={rows} rowKey={(row) => row.id} caption='플레이어 종합 랭킹' />
                        <Pager page={params.page} totalPages={totalPages} onPageChange={(page) => push({ page })} />
                    </>
                )}
            </PanelCard>
        </>
    )
}
