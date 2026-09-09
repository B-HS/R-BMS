import { Suspense, type PropsWithChildren } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { SessionGate } from '@widgets/auth/session-gate'

const AdminLayout = ({ children }: PropsWithChildren) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <SessionGate next='/admin/builds' requireAdmin>
            {children}
        </SessionGate>
    </Suspense>
)

export default AdminLayout
