'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import { useTranslations } from 'next-intl'
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

type SearchTranslator = ReturnType<typeof useTranslations<'browse.search'>>

const buildSortOptions = (t: SearchTranslator) => [
    { value: 'title', label: t('sort.title') },
    { value: 'level', label: t('sort.level') },
    { value: 'notes', label: t('sort.notes') },
    { value: 'recent', label: t('sort.recent') },
]

const buildColumnOptions = (t: SearchTranslator) => [
    { key: 'mode', label: t('columns.mode') },
    { key: 'level', label: t('columns.level') },
    { key: 'notes', label: t('columns.notes') },
    { key: 'bpm', label: 'BPM' },
    { key: 'md5', label: 'MD5' },
]

const DEFAULT_COLUMNS = ['mode', 'level', 'notes', 'bpm']
const FIXED_COLUMNS = ['title', 'artist']

const buildAllColumns = (t: SearchTranslator): DataTableColumn<ChartSearchRow>[] => [
    {
        key: 'title',
        label: t('columns.title'),
        flex: true,
        cell: (row) => (
            <Link href={`/charts/${row.sha256}`} className='font-medium hover:underline'>
                {formatChartTitle(row)}
            </Link>
        ),
    },
    { key: 'artist', label: t('columns.artist'), width: 160, hideBelow: 'sm', cell: (row) => row.artist || '—' },
    { key: 'mode', label: t('columns.mode'), width: 96, hideBelow: 'md', cell: (row) => row.mode },
    { key: 'level', label: t('columns.level'), width: 64, align: 'right', cell: (row) => row.level ?? '—' },
    { key: 'notes', label: t('columns.notes'), width: 96, align: 'right', hideBelow: 'sm', cell: (row) => formatCount(row.notes) },
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
    const t = useTranslations('browse.search')
    const { data, isLoading, isError, isFetching, refetch } = useChartSearch(params)

    const activeSort = params.sort ?? DEFAULT_SORT
    const activeColumns = (searchParams.get('cols') ?? DEFAULT_COLUMNS.join(',')).split(',').filter(Boolean)
    const columns = buildAllColumns(t).filter((column) => FIXED_COLUMNS.includes(column.key) || activeColumns.includes(column.key))
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
                sortOptions={buildSortOptions(t)}
                columnOptions={buildColumnOptions(t)}
                activeColumns={activeColumns}
                summary={isLoading ? t('searching') : t('resultCount', { count: data?.pagination?.total ?? rows.length })}
                onChange={(next) => push({ ...next, page: 1 })}
                onToggleColumn={(key) => push({ cols: toggleListValue(activeColumns, key) })}
            />
            {isLoading && (
                <div className='bg-card p-3'>
                    <TableSkeleton rows={SKELETON_ROWS} label={t('loading')} />
                </div>
            )}
            {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {!isLoading && !isError && rows.length === 0 && (
                <EmptyState
                    title={t('emptyTitle')}
                    description={hasFilter ? t('emptyFiltered') : t('emptyNone')}
                    action={
                        hasFilter && (
                            <Button variant='outline' size='sm' onClick={() => push({ q: undefined, mode: undefined, level: undefined, page: 1 })}>
                                {t('resetFilters')}
                            </Button>
                        )
                    }
                />
            )}
            {!isLoading && !isError && rows.length > 0 && (
                <div className='bg-card flex flex-col px-1 pb-2'>
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.sha256} caption={t('caption')} />
                    <div className='px-2'>
                        <Pager page={params.page} totalPages={totalPages} onPageChange={(page) => push({ page })} />
                    </div>
                </div>
            )}
        </>
    )
}
