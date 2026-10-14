import { Suspense } from 'react'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const FoldersLayout = ({ children }: LayoutProps<'/folders'>) => (
    <Suspense fallback={<ShellPageLoading title='내 폴더' />}>
        <SessionGate next='/folders'>{children}</SessionGate>
    </Suspense>
)

export default FoldersLayout
