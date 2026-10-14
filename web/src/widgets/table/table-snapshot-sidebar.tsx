'use client'

import type { FC } from 'react'
import { formatUrlHost } from '@features/table/format-url-host'
import { TableExternalLink } from '@features/table/table-external-link'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { formatCount } from '@shared/lib/format'
import { CopyButton } from '@shared/ui/copy-button'

type TableSnapshotSidebarProps = {
    table: TableSnapshotSummary
    tableUrl: string
    snapshotDate: string
}

const ROW_CLASS_NAME = 'flex items-baseline justify-between gap-2'

export const TableSnapshotSidebar: FC<TableSnapshotSidebarProps> = ({ table, tableUrl, snapshotDate }) => (
    <ShellSidebarPortal>
        <section aria-labelledby='table-sidebar-source' className='flex flex-col gap-2 px-3 py-3 text-xs'>
            <h2 id='table-sidebar-source' className='rbms-micro-label'>
                출처
            </h2>
            <dl className='flex flex-col gap-1'>
                <div className={ROW_CLASS_NAME}>
                    <dt className='text-muted-foreground'>보관본</dt>
                    <dd className='min-w-0 truncate'>
                        <TableExternalLink href={table.source.page_url} label={formatUrlHost(table.source.page_url)} appearance='text' />
                    </dd>
                </div>
                <div className={ROW_CLASS_NAME}>
                    <dt className='text-muted-foreground'>원본 갱신</dt>
                    <dd className='tabular-nums'>{table.last_update}</dd>
                </div>
                <div className={ROW_CLASS_NAME}>
                    <dt className='text-muted-foreground'>사본 수집</dt>
                    <dd className='tabular-nums'>{snapshotDate}</dd>
                </div>
                <div className={ROW_CLASS_NAME}>
                    <dt className='text-muted-foreground'>곡 · 레벨</dt>
                    <dd className='tabular-nums'>
                        {formatCount(table.entry_count)} · {formatCount(table.level_count)}
                    </dd>
                </div>
            </dl>
            <CopyButton
                value={tableUrl}
                label='표 주소 복사'
                copiedMessage='표 주소를 복사했습니다. R-BMS 의 난이도표 설정에 붙여 넣으세요.'
                showLabel
                className='self-start'
            />
        </section>
    </ShellSidebarPortal>
)
