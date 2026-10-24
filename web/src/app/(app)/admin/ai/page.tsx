import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiLimitsPanel } from '@widgets/admin/ai-limits-panel'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.adminAi')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const AdminAiPage = async () => {
    const t = await getTranslations('pages.route.adminAi')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <AiLimitsPanel />
        </ShellPage>
    )
}

export default AdminAiPage
