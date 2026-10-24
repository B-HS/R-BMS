import { getTranslations } from 'next-intl/server'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.notFound')
    return buildPageMetadata({ title: '404', description: t('description'), index: false })
}

const NotFoundPage = async () => {
    const t = await getTranslations('pages.route.notFound')

    return (
        <ShellPage title={t('title')}>
            <NotFoundNotice />
        </ShellPage>
    )
}

export default NotFoundPage
