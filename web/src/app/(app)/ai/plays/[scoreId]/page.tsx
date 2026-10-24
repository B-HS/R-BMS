import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiPlayDetailContent } from '@widgets/ai/ai-play-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.aiPlay')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const AiPlayPage = async ({ params }: PageProps<'/ai/plays/[scoreId]'>) => {
    const t = await getTranslations('pages.route.aiPlay')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <AiPlayDetailContent params={params} />
        </Suspense>
    )
}

export default AiPlayPage
