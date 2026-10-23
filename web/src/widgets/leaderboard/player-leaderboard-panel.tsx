'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
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
import { PLAYER_SORT_OPTIONS } from '@shared/constants/leaderboard'
import { INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'

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
            <div className='flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1'>
                <Link href={`/players/${encodeURIComponent(row.id)}`} className='font-medium hover:underline'>
                    {row.name}
                </Link>
                <DanBadgeList dans={row.dans} />
            </div>
        ),
    },
    { key: 'id', label: 'ID', width: 160, mono: true, hideBelow: 'sm', cell: (row) => row.id },
    { key: 'cleared_charts', label: '클리어', width: 96, align: 'right', cell: (row) => formatCount(row.cleared_charts) },
    { key: 'played_charts', label: '플레이 곡', width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.played_charts) },
    { key: 'total_plays', label: '제출 수', width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.total_plays) },
]

const LEADERBOARD_SCRATCH_AUTO_NOTE =
    '클리어·플레이 곡·제출 수는 기본으로 SCRATCH AUTO 판을 포함해 곡마다 더 높은 기록으로 셉니다. SCRATCH AUTO 포함을 끄면 일반 판만 셉니다. 클리어는 FAILED 가 아닌 모든 램프(ASSIST 포함)입니다.'

type PlayerLeaderboardPanelProps = {
    params: PlayerLeaderboardParams
}

export const PlayerLeaderboardPanel: FC<PlayerLeaderboardPanelProps> = ({ params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const { data, isLoading, isError, isFetching, refetch } = usePlayerLeaderboard(params)

    const rows = data?.data ?? []
    const totalPages = data?.pagination?.totalPages ?? 1
    const columns = buildColumns((params.page - 1) * params.limit)

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
            <FilterBar label='정렬' summary={isLoading ? '불러오는 중' : `플레이어 ${formatCount(data?.pagination?.total ?? rows.length)}명`}>
                <SortSelect label='정렬' value={params.sort} options={PLAYER_SORT_OPTIONS} onValueChange={(sort) => push({ sort, page: 1 })} />
                <ScratchAutoToggle
                    checked={params.include_scratch_auto}
                    onCheckedChange={(checked) => push({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(checked), page: 1 })}
                />
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
            <div className='bg-card px-3 py-2'>
                <PanelFootnote>{LEADERBOARD_SCRATCH_AUTO_NOTE}</PanelFootnote>
            </div>
        </>
    )
}
