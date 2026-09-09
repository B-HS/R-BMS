'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import { PanelCard } from '@features/shell/panel-card'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { Pager } from '@features/table/pager'
import { ColumnToggle } from '@features/table/column-toggle'
import { SortSelect } from '@features/table/sort-select'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { useChartSearch } from '@entities/chart.query'
import type { ChartSearchParams, ChartSearchRow } from '@entities/chart.type'
import { formatCount, formatHash } from '@shared/lib/format'
import { serializeSearchParams, toggleListValue } from '@shared/lib/search-params'

const DEFAULT_SORT = 'title'

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

const ALL_COLUMNS: DataTableColumn<ChartSearchRow>[] = [
    {
        key: 'title',
        label: '제목',
        flex: true,
        cell: (row) => (
            <Link href={`/charts/${row.sha256}`} className='hover:underline'>
                {row.title}
            </Link>
        ),
    },
    { key: 'artist', label: '아티스트', width: 160, flex: false, cell: (row) => row.artist },
    { key: 'mode', label: '모드', width: 96, cell: (row) => row.mode },
    { key: 'level', label: '레벨', width: 64, align: 'right', cell: (row) => row.level ?? '—' },
    { key: 'notes', label: '노트', width: 96, align: 'right', cell: (row) => formatCount(row.notes) },
    { key: 'bpm', label: 'BPM', width: 96, align: 'right', cell: (row) => (row.minbpm === row.maxbpm ? row.minbpm : `${row.minbpm}–${row.maxbpm}`) },
    { key: 'md5', label: 'MD5', width: 112, mono: true, cell: (row) => formatHash(row.md5) },
]

export const ChartSearchPanel: FC<{ params: ChartSearchParams }> = ({ params }) => {
    const router = useRouter()
    const pathname = usePathname()
    const searchParams = useSearchParams()
    const { data, isLoading, isError } = useChartSearch(params)

    const activeSort = params.sort ?? DEFAULT_SORT
    const activeColumns = (searchParams.get('cols') ?? DEFAULT_COLUMNS.join(',')).split(',').filter(Boolean)
    const columns = ALL_COLUMNS.filter((column) => column.key === 'title' || column.key === 'artist' || activeColumns.includes(column.key))
    const rows = data?.data ?? []
    const totalPages = data?.pagination?.totalPages ?? 1

    const push = (next: Record<string, string | number | string[] | undefined>) => {
        const merged = serializeSearchParams({
            q: searchParams.get('q') ?? undefined,
            mode: searchParams.get('mode') ?? undefined,
            level: searchParams.get('level') ?? undefined,
            sort: activeSort,
            page: params.page,
            cols: activeColumns,
            ...next,
        })
        router.push(merged ? `${pathname}?${merged}` : pathname)
    }

    return (
        <>
            <PanelCard title='검색 조건' contentClassName='flex flex-col gap-3'>
                <SortSelect label='정렬' value={activeSort} options={SORT_OPTIONS} onValueChange={(sort) => push({ sort, page: 1 })} />
                <ColumnToggle
                    options={COLUMN_OPTIONS}
                    selected={activeColumns}
                    onToggle={(key) => push({ cols: toggleListValue(activeColumns, key) })}
                />
            </PanelCard>

            <PanelCard title={`검색 결과 ${formatCount(data?.pagination?.total ?? rows.length)}건`} contentClassName='flex flex-col gap-2'>
                {isLoading && <TableSkeleton />}
                {!isLoading && isError && <LoadErrorState title='차트를 불러오지 못했습니다' />}
                {!isLoading && !isError && rows.length === 0 && (
                    <EmptyState title='조건에 맞는 차트가 없습니다' description='검색어나 필터를 조정해 보세요.' />
                )}
                {!isLoading && !isError && rows.length > 0 && (
                    <>
                        <DataTable columns={columns} rows={rows} rowKey={(row) => row.sha256} caption='차트 검색 결과' />
                        <Pager page={params.page} totalPages={totalPages} onPageChange={(page) => push({ page })} />
                    </>
                )}
            </PanelCard>
        </>
    )
}
