import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { TableDetailPanel } from '@widgets/table/table-detail-panel'
import { tableDetailQueryOptions } from '@entities/table.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'

export const TableDetailContent = async ({ params }: { params: Promise<{ tableId: string }> }) => {
    const { tableId } = await params

    const queryClient = getQueryClient()
    await fetchQueryOrNotFound(queryClient, tableDetailQueryOptions(tableId))

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <TableDetailPanel tableId={tableId} />
        </HydrationBoundary>
    )
}
