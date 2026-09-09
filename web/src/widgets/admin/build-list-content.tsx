import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { BuildListPanel } from '@widgets/admin/build-list-panel'
import { clientBuildListQueryOptions } from '@entities/build.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { forwardedCookieInit } from '@shared/lib/server-request-init'
import { PageHeading } from '@features/shell/page-heading'

export const BuildListContent = async () => {
    const queryClient = getQueryClient()
    await queryClient.prefetchQuery(clientBuildListQueryOptions(await forwardedCookieInit()))

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PageHeading>클라이언트 빌드 관리</PageHeading>
            <BuildListPanel />
        </HydrationBoundary>
    )
}
