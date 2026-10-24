'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { DashboardPanelEmpty } from '@features/dashboard/dashboard-panel-empty'
import { LoadErrorState } from '@features/state/load-error-state'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { useTableList } from '@entities/table.query'
import type { TableData } from '@entities/table.type'

const countCharts = (table: TableData) => table.folders.reduce((total, folder) => total + folder.charts.length, 0)

export const TableListPanel: FC = () => {
    const t = useTranslations('browse.table.list')
    const { data, isLoading, isError, isFetching, refetch } = useTableList()
    const rows = data ?? []

    if (isLoading) return <RowListSkeleton label={t('loading')} />
    if (isError) return <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
    if (rows.length === 0) return <DashboardPanelEmpty message={t('empty')} />

    return (
        <ul aria-label={t('listLabel')} className='bg-card flex flex-col border-t'>
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
                            {t('stats', { charts: countCharts(table), folders: table.folders.length, courses: table.courses.length })}
                        </span>
                    </div>
                </li>
            ))}
        </ul>
    )
}
