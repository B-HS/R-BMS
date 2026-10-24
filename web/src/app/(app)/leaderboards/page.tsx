import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { PlayerLeaderboardContent } from '@widgets/leaderboard/player-leaderboard-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.leaderboards')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/leaderboards' })
}

const LeaderboardsPage = async ({ searchParams }: PageProps<'/leaderboards'>) => {
    const t = await getTranslations('pages.route.leaderboards')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <Suspense fallback={<PanelSkeleton height='page' label={t('loading')} />}>
                <PlayerLeaderboardContent searchParams={searchParams} />
            </Suspense>
        </ShellPage>
    )
}

export default LeaderboardsPage
