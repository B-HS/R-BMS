'use client'

import { useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { FolderPlus, Plus } from 'lucide-react'
import { toast } from 'sonner'
import { ConfirmAction } from '@features/dialog/confirm-action'
import { FolderGuidePanel } from '@features/folder/folder-guide-panel'
import { FolderListItem } from '@features/folder/folder-list-item'
import { FolderMetaDialog } from '@features/folder/folder-meta-dialog'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'
import { useCreateFolder, useDeleteFolder, useGetFolderList, useReorderFolders, useUpdateFolder } from '@entities/folder.query'
import { FOLDER_LIMIT, toFolderErrorMessage, type FolderMetaInput, type FolderSummary } from '@entities/folder.type'
import { formatCount } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

const PAGE_TITLE = '내 폴더'
const PAGE_DESCRIPTION = 'R-BMS 선곡 화면에 표처럼 나타나는 곡 묶음'
const INSANE_TABLE_PATH = '/tables/insane1'
const EMPTY_META: FolderMetaInput = { name: '', description: '' }

const folderHref = (folderId: string) => `/folders/${encodeURIComponent(folderId)}`

export const FolderListPanel: FC = () => {
    const createButtonRef = useRef<HTMLButtonElement>(null)
    const returnFocusRef = useRef<HTMLElement | null>(null)
    const listRef = useRef<HTMLUListElement>(null)

    const [isCreateOpen, setIsCreateOpen] = useState(false)
    const [editTarget, setEditTarget] = useState<FolderSummary | null>(null)
    const [deleteTarget, setDeleteTarget] = useState<FolderSummary | null>(null)

    const router = useRouter()
    const { data, isLoading, isError, isFetching, refetch } = useGetFolderList()
    const createFolder = useCreateFolder()
    const updateFolder = useUpdateFolder()
    const deleteFolder = useDeleteFolder()
    const reorderFolders = useReorderFolders()

    const folders = data ?? []
    const isAtLimit = folders.length >= FOLDER_LIMIT.FOLDERS_PER_USER

    const openCreate = () => {
        createFolder.reset()
        returnFocusRef.current = null
        setIsCreateOpen(true)
    }

    const handleCreate = (input: FolderMetaInput) =>
        createFolder.mutate(input, {
            onSuccess: (created) => {
                setIsCreateOpen(false)
                toast.success(`'${created.name}' 폴더를 만들었습니다.`)
                router.push(folderHref(created.id))
            },
        })

    const handleUpdate = (input: FolderMetaInput) => {
        if (!editTarget) return
        updateFolder.mutate(
            { folderId: editTarget.id, input },
            {
                onSuccess: () => {
                    setEditTarget(null)
                    toast.success('이름과 설명을 고쳤습니다.')
                },
            },
        )
    }

    const handleDelete = () => {
        if (!deleteTarget) return
        const target = deleteTarget
        deleteFolder.mutate(target.id, {
            onSuccess: () => {
                setDeleteTarget(null)
                returnFocusRef.current = createButtonRef.current
                toast.success(`'${target.name}' 폴더를 지웠습니다.`)
            },
        })
    }

    const handleMove = (index: number, direction: 'up' | 'down') => {
        if (reorderFolders.isPending) return
        const target = direction === 'up' ? index - 1 : index + 1
        const ids = folders.map((folder) => folder.id)
        const moving = ids[index]
        const reordered = ids.toSpliced(index, 1).toSpliced(target, 0, moving)
        reorderFolders.mutate(reordered, {
            onSuccess: () =>
                requestAnimationFrame(() =>
                    listRef.current?.querySelector<HTMLButtonElement>(`[data-folder-id="${moving}"] [data-action="${direction}"]`)?.focus(),
                ),
        })
    }

    const createButton = (
        <Button ref={createButtonRef} size='sm' disabled={isAtLimit} aria-label='새 폴더' onClick={openCreate}>
            <Plus />
            <span className='hidden sm:inline'>새 폴더</span>
        </Button>
    )

    return (
        <ShellPage
            title={PAGE_TITLE}
            description={PAGE_DESCRIPTION}
            actions={
                <>
                    {data && (
                        <span
                            className='text-muted-foreground w-16 text-right text-xs tabular-nums data-full:text-destructive'
                            data-full={isAtLimit ? true : undefined}>
                            {formatCount(folders.length)} / {FOLDER_LIMIT.FOLDERS_PER_USER}
                        </span>
                    )}
                    {createButton}
                </>
            }>
            <ShellSidebarPortal>
                <FolderGuidePanel />
            </ShellSidebarPortal>
            {isLoading && <RowListSkeleton rows={3} label='폴더 불러오는 중' />}
            {!isLoading && isError && !data && (
                <LoadErrorState title='폴더를 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {data && folders.length === 0 && (
                <EmptyState
                    eyebrow='MY FOLDERS'
                    title='아직 폴더가 없습니다'
                    description='폴더를 만들고 곡을 넣으면 R-BMS 에 로그인했을 때 선곡 화면의 MY FOLDERS 아래에 나타납니다. 발광 난이도표에서 곡을 골라 넣을 수도 있습니다.'
                    className='min-h-64'
                    action={
                        <div className='flex flex-wrap justify-center gap-1'>
                            <Button size='sm' onClick={openCreate}>
                                <FolderPlus />첫 폴더 만들기
                            </Button>
                            <Button asChild variant='outline' size='sm'>
                                <Link href={INSANE_TABLE_PATH}>발광 난이도표 보기</Link>
                            </Button>
                        </div>
                    }
                />
            )}
            {folders.length > 0 && (
                <ul ref={listRef} aria-label='내 폴더 목록' aria-busy={reorderFolders.isPending} className='flex flex-col'>
                    {folders.map((folder, index) => (
                        <FolderListItem
                            key={folder.id}
                            folder={folder}
                            href={folderHref(folder.id)}
                            isFirst={index === 0}
                            isLast={index === folders.length - 1}
                            onMove={(direction) => handleMove(index, direction)}
                            onEdit={(trigger) => {
                                updateFolder.reset()
                                returnFocusRef.current = trigger
                                setEditTarget(folder)
                            }}
                            onDelete={(trigger) => {
                                returnFocusRef.current = trigger
                                setDeleteTarget(folder)
                            }}
                        />
                    ))}
                </ul>
            )}
            <FolderMetaDialog
                open={isCreateOpen}
                onOpenChange={setIsCreateOpen}
                title='새 폴더'
                description='이름은 R-BMS 선곡 화면에 그대로 보입니다. 곡은 만든 뒤 편집 화면에서 넣습니다.'
                submitLabel='만들기'
                initialValue={EMPTY_META}
                isPending={createFolder.isPending}
                serverError={createFolder.error ? toFolderErrorMessage(createFolder.error) : undefined}
                onSubmit={handleCreate}
            />
            <FolderMetaDialog
                open={editTarget !== null}
                onOpenChange={(open) => !open && setEditTarget(null)}
                title='이름·설명 고치기'
                submitLabel='고치기'
                initialValue={{ name: editTarget?.name ?? '', description: editTarget?.description ?? '' }}
                isPending={updateFolder.isPending}
                serverError={updateFolder.error ? toFolderErrorMessage(updateFolder.error) : undefined}
                onSubmit={handleUpdate}
                returnFocusRef={returnFocusRef}
            />
            <ConfirmAction
                open={deleteTarget !== null}
                onOpenChange={(open) => !open && setDeleteTarget(null)}
                title='폴더를 지울까요?'
                description={`'${deleteTarget?.name ?? ''}' 폴더와 그 안의 ${formatCount(deleteTarget?.item_count ?? 0)}곡 목록을 지웁니다. R-BMS 선곡 화면에서도 사라지며 되돌릴 수 없습니다.`}
                confirmLabel='지우기'
                confirmDisabled={deleteFolder.isPending}
                onConfirm={handleDelete}
                returnFocusRef={returnFocusRef}
            />
        </ShellPage>
    )
}
