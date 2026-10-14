import 'server-only'
import type { ComponentProps } from 'react'
import { connection } from 'next/server'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { readSessionUser } from '@widgets/auth/session-gate'
import { TableSnapshotWorkspace } from '@widgets/table/table-snapshot-workspace'
import { tableSnapshotRecordsQueryOptions } from '@entities/table-snapshot.query'
import { getQueryClient } from '@shared/utils/get-query-client'

type TableSnapshotSessionProps = Omit<ComponentProps<typeof TableSnapshotWorkspace>, 'initialUserId'>

const readSignedInUser = async () => {
    try {
        return await readSessionUser()
    } catch {
        return null
    }
}

/** Renders the table at request time so the address filters and the signed-in player's lamps arrive in the first HTML. */
export const TableSnapshotSession = async (props: TableSnapshotSessionProps) => {
    await connection()
    const user = await readSignedInUser()
    const queryClient = getQueryClient()

    if (user) {
        const { compose } = await import('@server/compose')
        await queryClient.prefetchQuery({
            ...tableSnapshotRecordsQueryOptions(props.detail.table.id, user.id),
            queryFn: async () =>
                (await compose().tableSnapshotService.getRecords({ tableId: props.detail.table.id, userId: user.id }))?.records ?? {},
        })
    }

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <TableSnapshotWorkspace {...props} initialUserId={user?.id ?? null} />
        </HydrationBoundary>
    )
}
