import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { PlayerLeaderboardContent } from '@widgets/leaderboard/player-leaderboard-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: '순위', description: 'R-BMS 점수 서버의 플레이어 종합 순위.', path: '/leaderboards' })

const LeaderboardsPage = ({ searchParams }: PageProps<'/leaderboards'>) => (
    <ShellPage title='순위' description='플레이어 종합 순위'>
        <Suspense fallback={<PanelSkeleton height='page' label='순위 불러오는 중' />}>
            <PlayerLeaderboardContent searchParams={searchParams} />
        </Suspense>
    </ShellPage>
)

export default LeaderboardsPage
