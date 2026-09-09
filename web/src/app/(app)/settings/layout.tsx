import { Suspense, type PropsWithChildren } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { SessionGate } from '@widgets/auth/session-gate'

const SettingsLayout = ({ children }: PropsWithChildren) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <SessionGate next='/settings'>{children}</SessionGate>
    </Suspense>
)

export default SettingsLayout
