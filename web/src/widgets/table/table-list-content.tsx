import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { TableListPanel } from '@widgets/table/table-list-panel'
import { tableListQueryOptions } from '@entities/table.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { PageHeading } from '@features/shell/page-heading'

export const TableListContent = async () => {
    const queryClient = getQueryClient()
    await queryClient.prefetchQuery(tableListQueryOptions())

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PageHeading>난이도표</PageHeading>
            <TableListPanel />
        </HydrationBoundary>
    )
}
