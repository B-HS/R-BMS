import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { SettingsContent } from '@widgets/settings/settings-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: '설정', description: '프로필, API 토큰, 동기화, 라이벌 설정.', index: false })

const SettingsPage = () => (
    <ShellPage title='설정' description='프로필 · API 토큰 · 동기화 · 라이벌'>
        <Suspense fallback={<PanelSkeleton height='page' label='설정 불러오는 중' />}>
            <SettingsContent />
        </Suspense>
    </ShellPage>
)

export default SettingsPage
