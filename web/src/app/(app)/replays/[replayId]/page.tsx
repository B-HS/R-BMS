import { Suspense } from 'react'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ReplayDetailContent } from '@widgets/replay/replay-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const metadata = buildPageMetadata({ title: '리플레이', description: 'R-BMS 리플레이의 입력 기록.', index: false })

const ReplayDetailPage = ({ params }: PageProps<'/replays/[replayId]'>) => (
    <Suspense fallback={<ShellPageLoading title='리플레이' />}>
        <ReplayDetailContent params={params} />
    </Suspense>
)

export default ReplayDetailPage
