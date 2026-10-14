import 'server-only'
import { notFound, redirect } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { readSessionUser } from '@widgets/auth/session-gate'
import { FolderEditor } from '@widgets/folder/folder-editor'
import { readFolderTableIndex } from '@widgets/folder/read-folder-table-index'
import { folderDetailQueryOptions } from '@entities/folder.query'
import { isFolderId } from '@entities/folder.type'
import { buildLoginHref } from '@shared/constants/auth-route'
import { getQueryClient } from '@shared/utils/get-query-client'
import { compose } from '@server/compose'

export const FolderEditorContent = async ({ params }: { params: Promise<{ folderId: string }> }) => {
    const { folderId } = await params
    const user = await readSessionUser()
    if (!user) redirect(buildLoginHref(`/folders/${encodeURIComponent(folderId)}`))
    if (!isFolderId(folderId)) notFound()

    const queryClient = getQueryClient()
    const { folderService } = compose()
    const folder = await folderService.get({ userId: user.id, folderId }).catch(() => undefined)
    if (folder === null) notFound()
    if (folder) await queryClient.prefetchQuery({ ...folderDetailQueryOptions(folderId), queryFn: () => folder })

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <FolderEditor folderId={folderId} tables={readFolderTableIndex()} />
        </HydrationBoundary>
    )
}
