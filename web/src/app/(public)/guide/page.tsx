import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { GuideArticle } from '@widgets/guide/guide-article'
import { GuideToc } from '@widgets/guide/guide-toc'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.guide')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/guide' })
}

const GuidePage = async () => {
    const t = await getTranslations('pages.route.guide')

    return (
        <ShellPage title={t('title')} description={t('description')} contentClassName='gap-0'>
            <GuideArticle />
            <ShellSidebarPortal>
                <GuideToc />
            </ShellSidebarPortal>
        </ShellPage>
    )
}

export default GuidePage
