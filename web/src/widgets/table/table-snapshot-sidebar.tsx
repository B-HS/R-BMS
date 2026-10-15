'use client'

import type { FC } from 'react'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { formatCount } from '@shared/lib/format'
import { CopyButton } from '@shared/ui/copy-button'

type TableSnapshotSidebarProps = {
    table: TableSnapshotSummary
    tableUrl: string
}

const ROW_CLASS_NAME = 'flex items-baseline justify-between gap-2'

export const TableSnapshotSidebar: FC<TableSnapshotSidebarProps> = ({ table, tableUrl }) => (
    <ShellSidebarPortal>
        <section aria-labelledby='table-sidebar-info' className='flex flex-col gap-2 px-3 py-3 text-xs'>
            <h2 id='table-sidebar-info' className='rbms-micro-label'>
                표 정보
            </h2>
            <dl className='flex flex-col gap-1'>
                <div className={ROW_CLASS_NAME}>
                    <dt className='text-muted-foreground'>갱신</dt>
                    <dd className='tabular-nums'>{table.last_update}</dd>
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
