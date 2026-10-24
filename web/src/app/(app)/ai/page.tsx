import { getTranslations } from 'next-intl/server'
import { AiPrincipleNote } from '@features/ai/ai-principle-note'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiHubTabs } from '@widgets/ai/ai-hub-tabs'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.ai')
    return buildPageMetadata({ title: 'AI', description: t('metaDescription'), index: false })
}

const AiPage = async () => {
    const t = await getTranslations('pages.route.ai')

    return (
        <ShellPage title='AI' description={t('description')}>
            <AiPrincipleNote />
            <AiHubTabs />
        </ShellPage>
    )
}

export default AiPage
