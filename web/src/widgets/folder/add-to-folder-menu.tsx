'use client'

import { useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { FolderOpen, FolderPlus, LogIn, RotateCw } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { FolderMetaDialog } from '@features/folder/folder-meta-dialog'
import { useSessionUser } from '@entities/auth.query'
import { useAddFolderItems, useCreateFolder, useGetFolderContaining, useGetFolderList, useRemoveFolderItems } from '@entities/folder.query'
import {
    FOLDER_LIMIT,
    isFolderMd5,
    toFolderErrorMessage,
    toNewFolderItemInput,
    type FolderMetaInput,
    type FolderSummary,
} from '@entities/folder.type'
import { buildLoginHref } from '@shared/constants/auth-route'
import { Button } from '@shared/ui/button'
import {
    DropdownMenu,
    DropdownMenuCheckboxItem,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@shared/ui/dropdown-menu'

const EMPTY_META: FolderMetaInput = { name: '', description: '' }

type AddToFolderMenuProps = {
    md5: string
    sha256?: string
    title?: string
    artist?: string
    level?: string
}

export const AddToFolderMenu: FC<AddToFolderMenuProps> = ({ md5, sha256, title, artist, level }) => {
    const triggerRef = useRef<HTMLButtonElement>(null)

    const [isOpen, setIsOpen] = useState(false)
    const [isCreateOpen, setIsCreateOpen] = useState(false)
    const [busyFolderId, setBusyFolderId] = useState<string | null>(null)

    const pathname = usePathname()
    const t = useTranslations('browse.folder.addMenu')
    const { user, isPending: isSessionPending } = useSessionUser()

    const chartMd5 = md5.trim().toLowerCase()
    const isChartValid = isFolderMd5(chartMd5)
    const canQuery = user !== null && isChartValid
    const itemInput = toNewFolderItemInput({ md5: chartMd5, sha256: sha256?.trim().toLowerCase(), title, artist })
    const chartLabel = title?.trim() || chartMd5

    const folderList = useGetFolderList(canQuery && isOpen)
    const containing = useGetFolderContaining(chartMd5, canQuery && isOpen)
    const addItems = useAddFolderItems()
    const removeItems = useRemoveFolderItems()
    const createFolder = useCreateFolder()

    const folders = folderList.data ?? []
    const containingIds = new Set(containing.data ?? [])
    const isListReady = folderList.data !== undefined && containing.data !== undefined
    const isListError = folderList.isError || containing.isError

    const toggleFolder = (folder: FolderSummary, shouldContain: boolean) => {
        setBusyFolderId(folder.id)
        const onSettled = () => setBusyFolderId(null)
        const onError = (error: Error) => toast.error(toFolderErrorMessage(error))
        if (shouldContain) {
            addItems.mutate(
                { folderId: folder.id, items: [itemInput] },
                { onSuccess: () => toast.success(t('added', { name: folder.name })), onError, onSettled },
            )
            return
        }
        removeItems.mutate(
            { folderId: folder.id, md5s: [chartMd5] },
            { onSuccess: () => toast.success(t('removed', { name: folder.name })), onError, onSettled },
        )
    }

    const handleCreate = (input: FolderMetaInput) =>
        createFolder.mutate(input, {
            onSuccess: (created) =>
                addItems.mutate(
                    { folderId: created.id, items: [itemInput] },
                    {
                        onSuccess: () => {
                            setIsCreateOpen(false)
                            toast.success(t('createdAndAdded', { name: created.name }))
                        },
                        onError: (error) => {
                            setIsCreateOpen(false)
                            toast.error(t('createdButNotAdded', { reason: toFolderErrorMessage(error) }))
                        },
                    },
                ),
        })

    if (!isChartValid) return null

    if (isSessionPending)
        return (
            <Button variant='outline' size='sm' disabled aria-label={t('trigger')}>
                <FolderPlus />
                <span className='hidden sm:inline'>{t('trigger')}</span>
            </Button>
        )

    if (!user)
        return (
            <Button asChild variant='outline' size='sm'>
                <Link href={buildLoginHref(pathname)} aria-label={t('signIn')}>
                    <LogIn />
                    <span className='hidden sm:inline'>{t('signIn')}</span>
                </Link>
            </Button>
        )

    return (
        <>
            <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
                <DropdownMenuTrigger asChild>
                    <Button ref={triggerRef} variant='outline' size='sm' aria-label={t('trigger')}>
                        <FolderPlus />
                        <span className='hidden sm:inline'>{t('trigger')}</span>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align='end' className='w-72'>
                    <DropdownMenuLabel className='flex min-w-0 flex-col gap-0.5'>
                        <span className='rbms-micro-label'>{t('title')}</span>
                        <span className='truncate text-xs font-normal'>
                            {level && <span className='mr-1.5 font-mono font-semibold'>{level}</span>}
                            {chartLabel}
                        </span>
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    {!isListReady && !isListError && (
                        <DropdownMenuItem disabled role='status'>
                            {t('loading')}
                        </DropdownMenuItem>
                    )}
                    {!isListReady && isListError && (
                        <DropdownMenuItem
                            onSelect={(event) => {
                                event.preventDefault()
                                void folderList.refetch()
                                void containing.refetch()
                            }}>
                            <RotateCw />
                            {t('retry')}
                        </DropdownMenuItem>
                    )}
                    {isListReady && folders.length === 0 && <DropdownMenuItem disabled>{t('noFolders')}</DropdownMenuItem>}
                    {isListReady &&
                        folders.map((folder) => {
                            const isContained = containingIds.has(folder.id)
                            const isFull = !isContained && folder.item_count >= FOLDER_LIMIT.ITEMS_PER_FOLDER
                            return (
                                <DropdownMenuCheckboxItem
                                    key={folder.id}
                                    checked={isContained}
                                    disabled={busyFolderId !== null || isFull}
                                    onSelect={(event) => event.preventDefault()}
                                    onCheckedChange={(checked) => toggleFolder(folder, checked === true)}>
                                    <span className='min-w-0 flex-1 truncate'>{folder.name}</span>
                                    <span className='text-muted-foreground text-xs tabular-nums'>
                                        {isFull ? t('full') : t('itemCount', { count: folder.item_count })}
                                    </span>
                                </DropdownMenuCheckboxItem>
                            )
                        })}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                        disabled={isListReady && folders.length >= FOLDER_LIMIT.FOLDERS_PER_USER}
                        onSelect={() => {
                            createFolder.reset()
                            setIsCreateOpen(true)
                        }}>
                        <FolderPlus />
                        {t('createAndAdd')}
                    </DropdownMenuItem>
                    <DropdownMenuItem asChild>
                        <Link href='/folders'>
                            <FolderOpen />
                            {t('manage')}
                        </Link>
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
            <FolderMetaDialog
                open={isCreateOpen}
                onOpenChange={setIsCreateOpen}
                title={t('dialogTitle')}
                description={t('dialogDescription', { chart: chartLabel })}
                submitLabel={t('dialogSubmit')}
                initialValue={EMPTY_META}
                isPending={createFolder.isPending || addItems.isPending}
                serverError={createFolder.error ? toFolderErrorMessage(createFolder.error) : undefined}
                onSubmit={handleCreate}
                returnFocusRef={triggerRef}
            />
        </>
    )
}
