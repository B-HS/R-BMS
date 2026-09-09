'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { PageHeading } from '@features/shell/page-heading'
import { MetaBadge } from '@features/badge/meta-badge'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { formatCount, formatHash } from '@shared/lib/format'
import { useTableDetail } from '@entities/table.query'
import type { TableChartRef } from '@entities/table.type'

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
    const { data, isLoading, isError } = useTableDetail(tableId)

    if (isLoading) return <PanelSkeleton height='page' />
    if (isError || !data) return <LoadErrorState title='표를 불러오지 못했습니다' />

    return (
        <>
            <PageHeading>{data.name}</PageHeading>
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
        </>
    )
}
