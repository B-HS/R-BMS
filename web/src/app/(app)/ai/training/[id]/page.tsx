import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiTrainingDetailContent } from '@widgets/ai/ai-training-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.aiTraining')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const AiTrainingPage = async ({ params }: PageProps<'/ai/training/[id]'>) => {
    const t = await getTranslations('pages.route.aiTraining')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <AiTrainingDetailContent params={params} />
        </Suspense>
    )
}

export default AiTrainingPage
