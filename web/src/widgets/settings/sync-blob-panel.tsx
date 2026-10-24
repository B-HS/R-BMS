'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { Button, buttonVariants } from '@shared/ui/button'
import { cn } from '@shared/lib/cn'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { useSettingBlob, useSettingKeyList } from '@entities/setting.query'
import type { SettingBlobKey } from '@entities/setting.type'

export const SyncBlobPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const [selectedKey, setSelectedKey] = useState('')

    const t = useTranslations('settings.syncBlobPanel')
    const { data, isLoading, isError, isFetching, refetch } = useSettingKeyList()
    const blob = useSettingBlob(playerId, selectedKey)

    const rows = data ?? []

    const columns: DataTableColumn<SettingBlobKey>[] = [
        { key: 'key', label: t('columnKey'), flex: true, mono: true, cell: (row) => row.key },
        { key: 'format', label: t('columnFormat'), width: 80, cell: (row) => row.format },
        { key: 'size', label: t('columnSize'), width: 96, align: 'right', cell: (row) => formatCount(row.size) },
        { key: 'updated_at', label: t('columnUpdatedAt'), width: 144, align: 'right', cell: (row) => formatDateTime(row.updated_at) },
        {
            key: 'action',
            label: '',
            width: 72,
            align: 'right',
            cell: (row) => (
                <Button variant='ghost' size='sm' onClick={() => setSelectedKey(row.key)}>
                    {t('view')}
                </Button>
            ),
        },
    ]

    return (
        <>
            <PanelCard title={t('title')} contentClassName='flex flex-col gap-2'>
                {isLoading && <TableSkeleton rows={3} />}
                {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
                {!isLoading && !isError && rows.length === 0 && <EmptyState title={t('emptyTitle')} description={t('emptyDescription')} />}
                {!isLoading && !isError && rows.length > 0 && (
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.key} caption={t('caption')} />
                )}
                <PanelFootnote>{t('footnote')}</PanelFootnote>
            </PanelCard>
            {selectedKey && (
                <PanelCard
                    title={t('contentTitle', { key: selectedKey })}
                    action={
                        <div className='flex items-center gap-2'>
                            {blob.data && (
                                <a
                                    className={cn(buttonVariants({ variant: 'ghost', size: 'sm' }))}
                                    download={`${selectedKey}.${blob.data.format}`}
                                    href={`data:text/plain;charset=utf-8,${encodeURIComponent(blob.data.content)}`}>
                                    {t('download')}
                                </a>
                            )}
                            <Button variant='ghost' size='sm' onClick={() => setSelectedKey('')}>
                                {t('close')}
                            </Button>
                        </div>
                    }
                    contentClassName='flex flex-col gap-2'>
                    {blob.isLoading && <TableSkeleton rows={5} />}
                    {!blob.isLoading && blob.isError && (
                        <LoadErrorState title={t('contentLoadError')} isRetrying={blob.isFetching} onRetry={() => void blob.refetch()} />
                    )}
                    {!blob.isLoading && !blob.isError && blob.data && (
                        <pre className='bg-muted max-h-96 overflow-auto p-2 font-mono text-xs whitespace-pre'>{blob.data.content}</pre>
                    )}
                </PanelCard>
            )}
        </>
    )
}
