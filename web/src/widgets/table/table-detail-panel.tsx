'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { PanelCard } from '@features/shell/panel-card'
import { MetaBadge } from '@features/badge/meta-badge'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { ApiError } from '@shared/lib/fetch'
import { formatHash } from '@shared/lib/format'
import { useTableDetail } from '@entities/table.query'
import type { TableChartRef } from '@entities/table.type'

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
    const t = useTranslations('browse.table.detail')
    const { data, isLoading, isError, error, isFetching, refetch } = useTableDetail(tableId)

    if (isLoading) return <ShellPageLoading title={t('pageTitle')} />

    if (error instanceof ApiError && error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={t('pageTitle')}>
                <NotFoundNotice
                    title={t('notFoundTitle')}
                    description={t('notFoundDescription')}
                    actionHref='/tables'
                    actionLabel={t('notFoundAction')}
                />
            </ShellPage>
        )

    if (isError || !data)
        return (
            <ShellPage title={t('pageTitle')}>
                <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />
            </ShellPage>
        )

    return (
        <ShellPage title={data.name} description={t('summary', { folders: data.folders.length, courses: data.courses.length })}>
            <PanelCard title={t('pageTitle')} contentClassName='flex flex-col gap-2'>
                <p className='text-muted-foreground text-xs'>{t('summary', { folders: data.folders.length, courses: data.courses.length })}</p>
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
            {data.folders.length === 0 && <EmptyState title={t('emptyFolders')} />}
            {data.folders.map((folder, index) => (
                <PanelCard key={`${index}-${folder.name}`} title={t('folderTitle', { name: folder.name, count: folder.charts.length })}>
                    <DataTable
                        columns={COLUMNS}
                        rows={folder.charts}
                        rowKey={(row) => row.sha256}
                        caption={t('folderCaption', { name: folder.name })}
                    />
                </PanelCard>
            ))}
        </ShellPage>
    )
}
