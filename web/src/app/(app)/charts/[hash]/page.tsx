import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ChartDetailContent } from '@widgets/chart/chart-detail-content'

const ChartDetailPage = ({ params }: { params: Promise<{ hash: string }> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <ChartDetailContent params={params} />
    </Suspense>
)

export default ChartDetailPage
