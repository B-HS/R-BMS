import { Suspense } from 'react'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const AiLayout = ({ children }: LayoutProps<'/ai'>) => (
    <Suspense fallback={<ShellPageLoading title='AI' />}>
        <SessionGate next='/ai'>{children}</SessionGate>
    </Suspense>
)

export default AiLayout
