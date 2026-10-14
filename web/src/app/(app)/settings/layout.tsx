import { Suspense } from 'react'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const SettingsLayout = ({ children }: LayoutProps<'/settings'>) => (
    <Suspense fallback={<ShellPageLoading title='설정' />}>
        <SessionGate next='/settings'>{children}</SessionGate>
    </Suspense>
)

export default SettingsLayout
