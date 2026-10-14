import { connection } from 'next/server'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { TableListPanel } from '@widgets/table/table-list-panel'
import { tableListQueryOptions } from '@entities/table.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readTables } from '@server/service/public-reads'

export const TableListContent = async () => {
    await connection()
    const queryClient = getQueryClient()
    await queryClient.prefetchQuery({ ...tableListQueryOptions(), queryFn: readTables })

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <TableListPanel />
        </HydrationBoundary>
    )
}
