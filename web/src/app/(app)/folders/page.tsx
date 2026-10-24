import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { FolderListContent } from '@widgets/folder/folder-list-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.folders')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const FoldersPage = async () => {
    const t = await getTranslations('pages.route.folders')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} description={t('description')} />}>
            <FolderListContent />
        </Suspense>
    )
}

export default FoldersPage
