'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { LoadErrorState } from '@features/state/load-error-state'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { formatCount } from '@shared/lib/format'
import { useTableList } from '@entities/table.query'
import type { TableData } from '@entities/table.type'

const countCharts = (table: TableData) => table.folders.reduce((total, folder) => total + folder.charts.length, 0)

export const TableListPanel: FC = () => {
    const { data, isLoading, isError, isFetching, refetch } = useTableList()
    const rows = data ?? []

    if (isLoading) return <RowListSkeleton label='난이도표 목록 불러오는 중' />
    if (isError) return <LoadErrorState title='표 목록을 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
    if (rows.length === 0) return <DashboardPanelEmpty message='서버에 따로 등록된 IR 난이도표가 아직 없습니다.' />

    return (
        <ul aria-label='서버 표 목록' className='bg-card flex flex-col border-t'>
            {rows.map((table) => (
                <li key={table.id} className='border-border hover:bg-muted/50 relative flex items-center gap-3 border-b px-3 py-2'>
                    <span aria-hidden className='bg-muted flex size-8 shrink-0 items-center justify-center text-sm font-semibold'>
                        {table.name.slice(0, 1)}
                    </span>
                    <div className='flex min-w-0 flex-col'>
                        <Link
                            href={`/tables/${encodeURIComponent(table.id)}`}
                            className='truncate text-sm font-medium outline-none after:absolute after:inset-0 focus-visible:underline'>
                            {table.name}
                        </Link>
                        <span className='text-muted-foreground truncate text-xs tabular-nums'>
                            차트 {formatCount(countCharts(table))} · 폴더 {formatCount(table.folders.length)} · 코스{' '}
                            {formatCount(table.courses.length)}
                        </span>
                    </div>
                </li>
            ))}
        </ul>
    )
}
