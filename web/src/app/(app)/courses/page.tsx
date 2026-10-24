import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { CourseCatalogContent } from '@widgets/course/course-catalog-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.courses')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/courses' })
}

const CoursesPage = async () => {
    const t = await getTranslations('pages.route.courses')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <Suspense fallback={<PanelSkeleton height='page' label={t('loading')} />}>
                <CourseCatalogContent />
            </Suspense>
        </ShellPage>
    )
}

export default CoursesPage
