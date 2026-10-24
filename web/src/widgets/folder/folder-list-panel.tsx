'use client'

import { useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { FolderPlus, Plus } from 'lucide-react'
import { useTranslations } from 'next-intl'
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
    const t = useTranslations('browse.folder')
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
                toast.success(t('list.created', { name: created.name }))
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
                    toast.success(t('list.updated'))
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
                toast.success(t('deleted', { name: target.name }))
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
        <Button ref={createButtonRef} size='sm' disabled={isAtLimit} aria-label={t('list.newFolder')} onClick={openCreate}>
            <Plus />
            <span className='hidden sm:inline'>{t('list.newFolder')}</span>
        </Button>
    )

    return (
        <ShellPage
            title={t('list.pageTitle')}
            description={t('list.pageDescription')}
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
            {isLoading && <RowListSkeleton rows={3} label={t('list.loading')} />}
            {!isLoading && isError && !data && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {data && folders.length === 0 && (
                <EmptyState
                    eyebrow='MY FOLDERS'
                    title={t('noFolders')}
                    description={t('list.emptyDescription')}
                    className='min-h-64'
                    action={
                        <div className='flex flex-wrap justify-center gap-1'>
                            <Button size='sm' onClick={openCreate}>
                                <FolderPlus />
                                {t('list.createFirst')}
                            </Button>
                            <Button asChild variant='outline' size='sm'>
                                <Link href={INSANE_TABLE_PATH}>{t('list.viewInsaneTable')}</Link>
                            </Button>
                        </div>
                    }
                />
            )}
            {folders.length > 0 && (
                <ul ref={listRef} aria-label={t('list.listLabel')} aria-busy={reorderFolders.isPending} className='flex flex-col'>
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
                title={t('list.newFolder')}
                description={t('list.createDescription')}
                submitLabel={t('list.create')}
                initialValue={EMPTY_META}
                isPending={createFolder.isPending}
                serverError={createFolder.error ? toFolderErrorMessage(createFolder.error) : undefined}
                onSubmit={handleCreate}
            />
            <FolderMetaDialog
                open={editTarget !== null}
                onOpenChange={(open) => !open && setEditTarget(null)}
                title={t('editor.editMeta')}
                submitLabel={t('list.edit')}
                initialValue={{ name: editTarget?.name ?? '', description: editTarget?.description ?? '' }}
                isPending={updateFolder.isPending}
                serverError={updateFolder.error ? toFolderErrorMessage(updateFolder.error) : undefined}
                onSubmit={handleUpdate}
                returnFocusRef={returnFocusRef}
            />
            <ConfirmAction
                open={deleteTarget !== null}
                onOpenChange={(open) => !open && setDeleteTarget(null)}
                title={t('deleteConfirmTitle')}
                description={t('deleteConfirmDescription', { name: deleteTarget?.name ?? '', count: deleteTarget?.item_count ?? 0 })}
                confirmLabel={t('deleteConfirm')}
                confirmDisabled={deleteFolder.isPending}
                onConfirm={handleDelete}
                returnFocusRef={returnFocusRef}
            />
        </ShellPage>
    )
}
