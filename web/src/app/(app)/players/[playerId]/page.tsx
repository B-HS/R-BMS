import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { readPlayerProfile } from '@server/service/public-reads'
import { PlayerDetailContent } from '@widgets/player/player-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async ({ params }: PageProps<'/players/[playerId]'>): Promise<Metadata> => {
    const { playerId } = await params
    const [t, player] = await Promise.all([getTranslations('pages.route.player'), readPlayerProfile(playerId).catch(() => null)])
    if (!player) return buildPageMetadata({ title: t('title'), index: false })
    return buildPageMetadata({
        title: player.name,
        description: t('metaDescription', { name: player.name, id: player.id }),
        path: `/players/${encodeURIComponent(player.id)}`,
    })
}

const PlayerDetailPage = async ({ params, searchParams }: PageProps<'/players/[playerId]'>) => {
    const t = await getTranslations('pages.route.player')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <PlayerDetailContent params={params} searchParams={searchParams} />
        </Suspense>
    )
}

export default PlayerDetailPage
