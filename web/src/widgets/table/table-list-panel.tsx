'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { formatCount } from '@shared/lib/format'
import { useTableList } from '@entities/table.query'
import type { TableData } from '@entities/table.type'

const countCharts = (table: TableData) => table.folders.reduce((total, folder) => total + folder.charts.length, 0)

const COLUMNS: DataTableColumn<TableData>[] = [
    {
        key: 'name',
        label: '표',
        flex: true,
        cell: (row) => (
            <Link href={`/tables/${row.id}`} className='hover:underline'>
                {row.name}
            </Link>
        ),
    },
    { key: 'folders', label: '폴더', width: 80, align: 'right', cell: (row) => formatCount(row.folders.length) },
    { key: 'charts', label: '차트', width: 96, align: 'right', cell: (row) => formatCount(countCharts(row)) },
    { key: 'courses', label: '코스', width: 80, align: 'right', cell: (row) => formatCount(row.courses.length) },
]

export const TableListPanel: FC = () => {
    const { data, isLoading, isError } = useTableList()
    const rows = data ?? []

    return (
        <PanelCard title='IR 난이도표' contentClassName='flex flex-col gap-2'>
            {isLoading && <TableSkeleton />}
            {!isLoading && isError && <LoadErrorState title='표 목록을 불러오지 못했습니다' />}
            {!isLoading && !isError && rows.length === 0 && <EmptyState title='등록된 표가 없습니다' />}
            {!isLoading && !isError && rows.length > 0 && (
                <DataTable columns={COLUMNS} rows={rows} rowKey={(row) => row.id} caption='서버가 큐레이션한 난이도표 목록' />
            )}
        </PanelCard>
    )
}
