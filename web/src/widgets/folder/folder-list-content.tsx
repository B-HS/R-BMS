import 'server-only'
import { redirect } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { readSessionUser } from '@widgets/auth/session-gate'
import { FolderListPanel } from '@widgets/folder/folder-list-panel'
import { folderListQueryOptions } from '@entities/folder.query'
import { buildLoginHref } from '@shared/constants/auth-route'
import { getQueryClient } from '@shared/utils/get-query-client'
import { compose } from '@server/compose'

const FOLDERS_PATH = '/folders'

export const FolderListContent = async () => {
    const user = await readSessionUser()
    if (!user) redirect(buildLoginHref(FOLDERS_PATH))

    const queryClient = getQueryClient()
    const { folderService } = compose()
    await queryClient.prefetchQuery({ ...folderListQueryOptions(), queryFn: () => folderService.list(user.id) })

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <FolderListPanel />
        </HydrationBoundary>
    )
}
