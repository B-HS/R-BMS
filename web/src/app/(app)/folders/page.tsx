import { Suspense } from 'react'
import { buildPageMetadata } from '@shared/lib/metadata'
import { FolderListContent } from '@widgets/folder/folder-list-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const metadata = buildPageMetadata({ title: '내 폴더', description: 'R-BMS 선곡 화면에 나타나는 내 곡 폴더.', index: false })

const FoldersPage = () => (
    <Suspense fallback={<ShellPageLoading title='내 폴더' description='R-BMS 선곡 화면에 표처럼 나타나는 곡 묶음' />}>
        <FolderListContent />
    </Suspense>
)

export default FoldersPage
