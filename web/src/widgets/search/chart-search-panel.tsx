'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { Pager } from '@features/table/pager'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { ChartSearchFilter } from '@widgets/search/chart-search-filter'
import { useChartSearch } from '@entities/chart.query'
import type { ChartSearchParams, ChartSearchRow } from '@entities/chart.type'
import { formatChartTitle, formatCount, formatHash } from '@shared/lib/format'
import { serializeSearchParams, toggleListValue } from '@shared/lib/search-params'
import { Button } from '@shared/ui/button'

const DEFAULT_SORT = 'title'
const SKELETON_ROWS = 10

const SORT_OPTIONS = [
    { value: 'title', label: '제목 순' },
    { value: 'level', label: '레벨 순' },
    { value: 'notes', label: '노트 많은 순' },
    { value: 'recent', label: '최근 등록 순' },
]

const COLUMN_OPTIONS = [
    { key: 'mode', label: '모드' },
    { key: 'level', label: '레벨' },
    { key: 'notes', label: '노트' },
    { key: 'bpm', label: 'BPM' },
    { key: 'md5', label: 'MD5' },
]

const DEFAULT_COLUMNS = ['mode', 'level', 'notes', 'bpm']
const FIXED_COLUMNS = ['title', 'artist']

const ALL_COLUMNS: DataTableColumn<ChartSearchRow>[] = [
    {
        key: 'title',
        label: '제목',
        flex: true,
        cell: (row) => (
            <Link href={`/charts/${row.sha256}`} className='font-medium hover:underline'>
                {formatChartTitle(row)}
            </Link>
        ),
    },
    { key: 'artist', label: '아티스트', width: 160, hideBelow: 'sm', cell: (row) => row.artist || '—' },
    { key: 'mode', label: '모드', width: 96, hideBelow: 'md', cell: (row) => row.mode },
    { key: 'level', label: '레벨', width: 64, align: 'right', cell: (row) => row.level ?? '—' },
    { key: 'notes', label: '노트', width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.notes) },
    {
        key: 'bpm',
        label: 'BPM',
        width: 96,
        align: 'right',
        hideBelow: 'md',
        cell: (row) => (row.minbpm === row.maxbpm ? row.minbpm : `${row.minbpm}–${row.maxbpm}`),
    },
    { key: 'md5', label: 'MD5', width: 112, mono: true, hideBelow: 'md', cell: (row) => formatHash(row.md5) },
]

export const ChartSearchPanel: FC<{ params: ChartSearchParams }> = ({ params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const searchParams = useSearchParams()
    const { data, isLoading, isError, isFetching, refetch } = useChartSearch(params)

    const activeSort = params.sort ?? DEFAULT_SORT
    const activeColumns = (searchParams.get('cols') ?? DEFAULT_COLUMNS.join(',')).split(',').filter(Boolean)
    const columns = ALL_COLUMNS.filter((column) => FIXED_COLUMNS.includes(column.key) || activeColumns.includes(column.key))
    const rows = data?.data ?? []
    const totalPages = data?.pagination?.totalPages ?? 1
    const hasFilter = Boolean(params.q || params.mode || params.level)

    const push = (next: Record<string, string | number | string[] | undefined>) => {
        const merged = serializeSearchParams({
            q: params.q,
            mode: params.mode,
            level: params.level,
            sort: activeSort,
            page: params.page,
            cols: activeColumns,
            ...next,
        })
        router.push(merged ? `${pathname}?${merged}` : pathname)
    }

    return (
        <>
            <ChartSearchFilter
                key={`${params.q ?? ''}|${params.level ?? ''}`}
                keyword={params.q ?? ''}
                mode={params.mode ?? ''}
                level={params.level ?? ''}
                sort={activeSort}
                sortOptions={SORT_OPTIONS}
                columnOptions={COLUMN_OPTIONS}
                activeColumns={activeColumns}
                summary={isLoading ? '검색 중' : `${formatCount(data?.pagination?.total ?? rows.length)}건`}
                onChange={(next) => push({ ...next, page: 1 })}
                onToggleColumn={(key) => push({ cols: toggleListValue(activeColumns, key) })}
            />
            {isLoading && (
                <div className='bg-card p-3'>
                    <TableSkeleton rows={SKELETON_ROWS} label='차트 검색 결과 불러오는 중' />
                </div>
            )}
            {!isLoading && isError && <LoadErrorState title='차트를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && rows.length === 0 && (
                <EmptyState
                    title='조건에 맞는 차트가 없습니다'
                    description={hasFilter ? '검색어나 필터를 바꿔 보세요.' : '아직 등록된 차트가 없습니다.'}
                    action={
                        hasFilter && (
                            <Button variant='outline' size='sm' onClick={() => push({ q: undefined, mode: undefined, level: undefined, page: 1 })}>
                                필터 초기화
                            </Button>
                        )
                    }
                />
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <div className='bg-card flex flex-col px-1 pb-2'>
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.sha256} caption='차트 검색 결과' />
                    <div className='px-2'>
                        <Pager page={params.page} totalPages={totalPages} onPageChange={(page) => push({ page })} />
                    </div>
                </div>
            )}
        </>
    )
}
