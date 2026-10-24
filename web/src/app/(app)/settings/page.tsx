import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { SettingsContent } from '@widgets/settings/settings-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.settings')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const SettingsPage = async () => {
    const t = await getTranslations('pages.route.settings')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <Suspense fallback={<PanelSkeleton height='page' label={t('loading')} />}>
                <SettingsContent />
            </Suspense>
        </ShellPage>
    )
}

export default SettingsPage
