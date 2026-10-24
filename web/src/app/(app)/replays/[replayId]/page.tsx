import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ReplayDetailContent } from '@widgets/replay/replay-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.replay')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), index: false })
}

const ReplayDetailPage = async ({ params }: PageProps<'/replays/[replayId]'>) => {
    const t = await getTranslations('pages.route.replay')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <ReplayDetailContent params={params} />
        </Suspense>
    )
}

export default ReplayDetailPage
