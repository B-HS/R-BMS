import { Suspense } from 'react'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const AdminLayout = ({ children }: LayoutProps<'/admin'>) => (
    <Suspense fallback={<ShellPageLoading title='관리' />}>
        <SessionGate next='/admin/builds' requireAdmin>
            {children}
        </SessionGate>
    </Suspense>
)

export default AdminLayout
