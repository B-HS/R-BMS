import { Suspense } from 'react'
import type { Metadata } from 'next'
import { buildPageMetadata } from '@shared/lib/metadata'
import { readPlayerProfile } from '@server/service/public-reads'
import { PlayerDetailContent } from '@widgets/player/player-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async ({ params }: PageProps<'/players/[playerId]'>): Promise<Metadata> => {
    const { playerId } = await params
    const player = await readPlayerProfile(playerId).catch(() => null)
    if (!player) return buildPageMetadata({ title: '플레이어', index: false })
    return buildPageMetadata({
        title: player.name,
        description: `${player.name} (@${player.id}) 의 R-BMS 기록과 램프 분포.`,
        path: `/players/${encodeURIComponent(player.id)}`,
    })
}

const PlayerDetailPage = ({ params }: PageProps<'/players/[playerId]'>) => (
    <Suspense fallback={<ShellPageLoading title='플레이어' />}>
        <PlayerDetailContent params={params} />
    </Suspense>
)

export default PlayerDetailPage
