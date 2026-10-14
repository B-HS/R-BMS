import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { BuildListContent } from '@widgets/admin/build-list-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: '빌드 관리', description: '클라이언트 빌드 허용 목록.', index: false })

const AdminBuildsPage = () => (
    <ShellPage title='빌드 관리' description='순위에 반영할 클라이언트 빌드'>
        <Suspense fallback={<PanelSkeleton height='page' label='빌드 목록 불러오는 중' />}>
            <BuildListContent />
        </Suspense>
    </ShellPage>
)

export default AdminBuildsPage
