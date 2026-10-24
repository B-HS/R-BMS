'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { TableSnapshotCard } from '@features/table/table-snapshot-card'
import { summariseTableRecords } from '@widgets/table/table-snapshot-filter'
import { useSessionUser } from '@entities/auth.query'
import { useGetTableSnapshotRecords } from '@entities/table-snapshot.query'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { CopyButton } from '@shared/ui/copy-button'

type TableSnapshotListItemProps = {
    table: TableSnapshotSummary
    tableUrl: string
}

export const TableSnapshotListItem: FC<TableSnapshotListItemProps> = ({ table, tableUrl }) => {
    const t = useTranslations('browse.table')
    const { user, isPending: isSessionPending } = useSessionUser()
    const recordsQuery = useGetTableSnapshotRecords(table.id, user?.id ?? null)
    const records = recordsQuery.data

    return (
        <li className='min-w-0'>
            <TableSnapshotCard
                table={table}
                progress={records ? summariseTableRecords(records, table.entry_count) : null}
                isProgressPending={isSessionPending || (user !== null && recordsQuery.isPending)}
                isSignedIn={user !== null}
                action={<CopyButton value={tableUrl} label={t('copyTableUrl')} copiedMessage={t('copiedTableUrl')} />}
            />
            {table.source && (
                <p className='bg-card text-muted-foreground -mt-1 px-3 pb-2 text-xs'>
                    {t('source', { name: table.source.name, maker: table.source.maker })}
                </p>
            )}
        </li>
    )
}
