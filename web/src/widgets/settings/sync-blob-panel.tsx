'use client'

import { useState, type FC } from 'react'
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

    const { data, isLoading, isError } = useSettingKeyList()
    const blob = useSettingBlob(playerId, selectedKey)

    const rows = data ?? []

    const columns: DataTableColumn<SettingBlobKey>[] = [
        { key: 'key', label: '키', flex: true, mono: true, cell: (row) => row.key },
        { key: 'format', label: '포맷', width: 80, cell: (row) => row.format },
        { key: 'size', label: '크기', width: 96, align: 'right', cell: (row) => formatCount(row.size) },
        { key: 'updated_at', label: '갱신', width: 144, align: 'right', cell: (row) => formatDateTime(row.updated_at) },
        {
            key: 'action',
            label: '',
            width: 72,
            align: 'right',
            cell: (row) => (
                <Button variant='ghost' size='sm' onClick={() => setSelectedKey(row.key)}>
                    보기
                </Button>
            ),
        },
    ]

    return (
        <>
            <PanelCard title='동기화 blob' contentClassName='flex flex-col gap-2'>
                {isLoading && <TableSkeleton rows={3} />}
                {!isLoading && isError && <LoadErrorState title='설정 목록을 불러오지 못했습니다' />}
                {!isLoading && !isError && rows.length === 0 && (
                    <EmptyState title='동기화된 설정이 없습니다' description='클라이언트에서 설정을 업로드하면 여기에 표시됩니다.' />
                )}
                {!isLoading && !isError && rows.length > 0 && (
                    <DataTable columns={columns} rows={rows} rowKey={(row) => row.key} caption='동기화된 설정 blob 목록' />
                )}
                <PanelFootnote>웹에서는 읽기 전용입니다. 수정은 클라이언트가 PUT 으로 반영합니다.</PanelFootnote>
            </PanelCard>
            {selectedKey && (
                <PanelCard
                    title={`${selectedKey} 내용`}
                    action={
                        <div className='flex items-center gap-2'>
                            {blob.data && (
                                <a
                                    className={cn(buttonVariants({ variant: 'ghost', size: 'sm' }))}
                                    download={`${selectedKey}.${blob.data.format}`}
                                    href={`data:text/plain;charset=utf-8,${encodeURIComponent(blob.data.content)}`}>
                                    다운로드
                                </a>
                            )}
                            <Button variant='ghost' size='sm' onClick={() => setSelectedKey('')}>
                                닫기
                            </Button>
                        </div>
                    }
                    contentClassName='flex flex-col gap-2'>
                    {blob.isLoading && <TableSkeleton rows={5} />}
                    {!blob.isLoading && blob.isError && <LoadErrorState title='설정 내용을 불러오지 못했습니다' />}
                    {!blob.isLoading && !blob.isError && blob.data && (
                        <pre className='bg-muted max-h-96 overflow-auto p-2 font-mono text-xs whitespace-pre'>{blob.data.content}</pre>
                    )}
                </PanelCard>
            )}
        </>
    )
}
