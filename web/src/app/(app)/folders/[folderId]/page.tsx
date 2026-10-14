import { Suspense } from 'react'
import { buildPageMetadata } from '@shared/lib/metadata'
import { FolderEditorContent } from '@widgets/folder/folder-editor-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const metadata = buildPageMetadata({ title: '폴더 편집', description: '내 곡 폴더의 곡과 순서를 고칩니다.', index: false })

const FolderEditorPage = ({ params }: PageProps<'/folders/[folderId]'>) => (
    <Suspense fallback={<ShellPageLoading title='폴더' />}>
        <FolderEditorContent params={params} />
    </Suspense>
)

export default FolderEditorPage
