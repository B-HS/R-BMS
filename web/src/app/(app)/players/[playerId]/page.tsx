import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { PlayerDetailContent } from '@widgets/player/player-detail-content'

const PlayerDetailPage = ({ params }: { params: Promise<{ playerId: string }> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <PlayerDetailContent params={params} />
    </Suspense>
)

export default PlayerDetailPage
