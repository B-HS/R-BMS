import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { TableDetailContent } from '@widgets/table/table-detail-content'

const TableDetailPage = ({ params }: { params: Promise<{ tableId: string }> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <TableDetailContent params={params} />
    </Suspense>
)

export default TableDetailPage
