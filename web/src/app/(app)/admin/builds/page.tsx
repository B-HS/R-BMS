import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { BuildListContent } from '@widgets/admin/build-list-content'

const AdminBuildsPage = () => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <BuildListContent />
    </Suspense>
)

export default AdminBuildsPage
