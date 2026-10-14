import { redirect } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { BuildListPanel } from '@widgets/admin/build-list-panel'
import { ADMIN_ROLE, readSessionUser } from '@widgets/auth/session-gate'
import { clientBuildListQueryOptions } from '@entities/build.query'
import { DEFAULT_NEXT_PATH } from '@shared/constants/auth-route'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readClientBuilds } from '@server/service/public-reads'

export const BuildListContent = async () => {
    const user = await readSessionUser()
    if (user?.role !== ADMIN_ROLE) redirect(DEFAULT_NEXT_PATH)

    const queryClient = getQueryClient()
    await queryClient.prefetchQuery({ ...clientBuildListQueryOptions(), queryFn: readClientBuilds })

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <BuildListPanel />
        </HydrationBoundary>
    )
}
