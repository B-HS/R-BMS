import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { FolderEditorContent } from '@widgets/folder/folder-editor-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.folderEditor')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const FolderEditorPage = async ({ params }: PageProps<'/folders/[folderId]'>) => {
    const t = await getTranslations('pages.route.folderEditor')

    return (
        <Suspense fallback={<ShellPageLoading title={t('loadingTitle')} />}>
            <FolderEditorContent params={params} />
        </Suspense>
    )
}

export default FolderEditorPage
