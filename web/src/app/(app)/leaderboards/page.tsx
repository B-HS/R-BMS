import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { PlayerLeaderboardContent } from '@widgets/leaderboard/player-leaderboard-content'

const LeaderboardsPage = ({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <PlayerLeaderboardContent searchParams={searchParams} />
    </Suspense>
)

export default LeaderboardsPage
