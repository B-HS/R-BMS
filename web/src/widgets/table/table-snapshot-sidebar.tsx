'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { formatCount } from '@shared/lib/format'
import { CopyButton } from '@shared/ui/copy-button'

type TableSnapshotSidebarProps = {
    table: TableSnapshotSummary
    tableUrl: string
}

const ROW_CLASS_NAME = 'flex items-baseline justify-between gap-2'

export const TableSnapshotSidebar: FC<TableSnapshotSidebarProps> = ({ table, tableUrl }) => {
    const t = useTranslations('browse.table')

    return (
        <ShellSidebarPortal>
            <section aria-labelledby='table-sidebar-info' className='flex flex-col gap-2 px-3 py-3 text-xs'>
                <h2 id='table-sidebar-info' className='rbms-micro-label'>
                    {t('sidebar.heading')}
                </h2>
                <dl className='flex flex-col gap-1'>
                    <div className={ROW_CLASS_NAME}>
                        <dt className='text-muted-foreground'>{t('sidebar.updated')}</dt>
                        <dd className='tabular-nums'>{table.last_update}</dd>
                    </div>
                    <div className={ROW_CLASS_NAME}>
                        <dt className='text-muted-foreground'>{t('sidebar.chartsAndLevels')}</dt>
                        <dd className='tabular-nums'>
                            {formatCount(table.entry_count)} · {formatCount(table.level_count)}
                        </dd>
                    </div>
                </dl>
                <CopyButton value={tableUrl} label={t('copyTableUrl')} copiedMessage={t('copiedTableUrl')} showLabel className='self-start' />
            </section>
        </ShellSidebarPortal>
    )
}
