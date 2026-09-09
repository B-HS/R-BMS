import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { SettingsContent } from '@widgets/settings/settings-content'

const SettingsPage = () => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <SettingsContent />
    </Suspense>
)

export default SettingsPage
