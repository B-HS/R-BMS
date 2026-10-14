'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { MetaBadge } from '@features/badge/meta-badge'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { ApiError } from '@shared/lib/fetch'
import { formatCount, formatHash } from '@shared/lib/format'
import { useTableDetail } from '@entities/table.query'
import type { TableChartRef } from '@entities/table.type'

const PAGE_TITLE = '난이도표'
const NOT_FOUND_STATUS = 404

const COLUMNS: DataTableColumn<TableChartRef>[] = [
    {
        key: 'sha256',
        label: 'sha256',
        flex: true,
        mono: true,
        cell: (row) => (
            <Link href={`/charts/${row.sha256}`} className='hover:underline'>
                {formatHash(row.sha256)}
            </Link>
        ),
    },
    { key: 'md5', label: 'md5', width: 128, mono: true, cell: (row) => formatHash(row.md5) },
]

export const TableDetailPanel: FC<{ tableId: string }> = ({ tableId }) => {
    const { data, isLoading, isError, error, isFetching, refetch } = useTableDetail(tableId)

    if (isLoading) return <ShellPageLoading title={PAGE_TITLE} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={PAGE_TITLE}>
                <NotFoundNotice
                    title='없는 난이도표입니다'
                    description='이 주소로 등록된 표가 없습니다.'
                    actionHref='/tables'
                    actionLabel='난이도표 목록'
                />
            </ShellPage>
        )

    if (isError || !data)
        return (
            <ShellPage title={PAGE_TITLE}>
                <LoadErrorState title='표를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
            </ShellPage>
        )

    return (
        <ShellPage title={data.name} description={`폴더 ${formatCount(data.folders.length)}개 · 코스 ${formatCount(data.courses.length)}개`}>
            <PanelCard title='난이도표' contentClassName='flex flex-col gap-2'>
                <p className='text-muted-foreground text-xs'>
                    폴더 {formatCount(data.folders.length)}개 · 코스 {formatCount(data.courses.length)}개
                </p>
                {data.courses.length > 0 && (
                    <div className='flex flex-wrap gap-1'>
                        {data.courses.map((course) => (
                            <Link key={course.course_hash} href={`/courses/${course.course_hash}`} className='hover:underline'>
                                <MetaBadge>{course.name || course.course_hash}</MetaBadge>
                            </Link>
                        ))}
                    </div>
                )}
            </PanelCard>
            {data.folders.length === 0 && <EmptyState title='폴더가 없습니다' />}
            {data.folders.map((folder, index) => (
                <PanelCard key={`${index}-${folder.name}`} title={`${folder.name} (${formatCount(folder.charts.length)})`}>
                    <DataTable columns={COLUMNS} rows={folder.charts} rowKey={(row) => row.sha256} caption={`${folder.name} 폴더의 차트`} />
                </PanelCard>
            ))}
        </ShellPage>
    )
}
