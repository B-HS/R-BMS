'use client'

import { useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { FolderOpen, FolderPlus, LogIn, RotateCw } from 'lucide-react'
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
import { formatCount } from '@shared/lib/format'
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

const TRIGGER_LABEL = '내 폴더에 넣기'
const SIGN_IN_LABEL = '로그인하고 폴더에 넣기'
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
                { onSuccess: () => toast.success(`'${folder.name}' 에 넣었습니다.`), onError, onSettled },
            )
            return
        }
        removeItems.mutate(
            { folderId: folder.id, md5s: [chartMd5] },
            { onSuccess: () => toast.success(`'${folder.name}' 에서 뺐습니다.`), onError, onSettled },
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
                            toast.success(`'${created.name}' 폴더를 만들고 곡을 넣었습니다.`)
                        },
                        onError: (error) => {
                            setIsCreateOpen(false)
                            toast.error(`폴더는 만들었지만 곡을 넣지 못했습니다. ${toFolderErrorMessage(error)}`)
                        },
                    },
                ),
        })

    if (!isChartValid) return null

    if (isSessionPending)
        return (
            <Button variant='outline' size='sm' disabled aria-label={TRIGGER_LABEL}>
                <FolderPlus />
                <span className='hidden sm:inline'>{TRIGGER_LABEL}</span>
            </Button>
        )

    if (!user)
        return (
            <Button asChild variant='outline' size='sm'>
                <Link href={buildLoginHref(pathname)} aria-label={SIGN_IN_LABEL}>
                    <LogIn />
                    <span className='hidden sm:inline'>{SIGN_IN_LABEL}</span>
                </Link>
            </Button>
        )

    return (
        <>
            <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
                <DropdownMenuTrigger asChild>
                    <Button ref={triggerRef} variant='outline' size='sm' aria-label={TRIGGER_LABEL}>
                        <FolderPlus />
                        <span className='hidden sm:inline'>{TRIGGER_LABEL}</span>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align='end' className='w-72'>
                    <DropdownMenuLabel className='flex min-w-0 flex-col gap-0.5'>
                        <span className='rbms-micro-label'>내 폴더</span>
                        <span className='truncate text-xs font-normal'>
                            {level && <span className='mr-1.5 font-mono font-semibold'>{level}</span>}
                            {chartLabel}
                        </span>
                    </DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    {!isListReady && !isListError && (
                        <DropdownMenuItem disabled role='status'>
                            불러오는 중
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
                            불러오지 못했습니다. 다시 시도
                        </DropdownMenuItem>
                    )}
                    {isListReady && folders.length === 0 && <DropdownMenuItem disabled>아직 폴더가 없습니다</DropdownMenuItem>}
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
                                        {isFull ? '가득 참' : `${formatCount(folder.item_count)}곡`}
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
                        <FolderPlus />새 폴더를 만들어 넣기
                    </DropdownMenuItem>
                    <DropdownMenuItem asChild>
                        <Link href='/folders'>
                            <FolderOpen />내 폴더 관리
                        </Link>
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
            <FolderMetaDialog
                open={isCreateOpen}
                onOpenChange={setIsCreateOpen}
                title='새 폴더에 넣기'
                description={`'${chartLabel}' 을 새 폴더에 넣습니다.`}
                submitLabel='만들고 넣기'
                initialValue={EMPTY_META}
                isPending={createFolder.isPending || addItems.isPending}
                serverError={createFolder.error ? toFolderErrorMessage(createFolder.error) : undefined}
                onSubmit={handleCreate}
                returnFocusRef={triggerRef}
            />
        </>
    )
}
