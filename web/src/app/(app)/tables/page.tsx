import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { TableListContent } from '@widgets/table/table-list-content'

const TablesPage = () => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <TableListContent />
    </Suspense>
)

export default TablesPage
