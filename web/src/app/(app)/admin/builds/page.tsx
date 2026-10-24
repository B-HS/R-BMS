import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { BuildListContent } from '@widgets/admin/build-list-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.adminBuilds')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const AdminBuildsPage = async () => {
    const t = await getTranslations('pages.route.adminBuilds')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <Suspense fallback={<PanelSkeleton height='page' label={t('loading')} />}>
                <BuildListContent />
            </Suspense>
        </ShellPage>
    )
}

export default AdminBuildsPage
