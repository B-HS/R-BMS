import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ReplayDetailContent } from '@widgets/replay/replay-detail-content'

const ReplayDetailPage = ({ params }: { params: Promise<{ replayId: string }> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <ReplayDetailContent params={params} />
    </Suspense>
)

export default ReplayDetailPage
