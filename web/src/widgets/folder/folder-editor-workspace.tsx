'use client'

import { useEffect, useEffectEvent, useRef, useState, type FC } from 'react'
import { useRouter } from 'next/navigation'
import { Ellipsis, Import, Pencil, Plus, Save, Trash2, Undo2 } from 'lucide-react'
import { toast } from 'sonner'
import { ConfirmAction } from '@features/dialog/confirm-action'
import { FolderChartRow, type FolderChartMove } from '@features/folder/folder-chart-row'
import { FolderConflictNotice } from '@features/folder/folder-conflict-notice'
import { FolderGuidePanel } from '@features/folder/folder-guide-panel'
import { FolderMetaDialog } from '@features/folder/folder-meta-dialog'
import { FolderNoteDialog } from '@features/folder/folder-note-dialog'
import { FolderTableImportDialog } from '@features/folder/folder-table-import-dialog'
import { EmptyState } from '@features/state/empty-state'
import { FolderChartSearchPanel } from '@widgets/folder/folder-chart-search-panel'
import {
    appendFolderItems,
    buildLevelLabelByMd5,
    countFolderDraftChanges,
    describeFolderDraftChanges,
    moveFolderItem,
    summariseFolderConflict,
    toFolderBase,
    toFolderDraft,
    toReplaceItems,
    type FolderBase,
} from '@widgets/folder/folder-draft'
import { useFolderLeaveGuard } from '@widgets/folder/use-folder-leave-guard'
import { ShellPageHeader } from '@widgets/shell/shell-page-header'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'
import { useDeleteFolder, useReplaceFolderItems, useUpdateFolder } from '@entities/folder.query'
import {
    FOLDER_ERROR_CODE,
    FOLDER_LIMIT,
    getFolderErrorCode,
    toFolderErrorMessage,
    type FolderDetail,
    type FolderItem,
    type FolderTableIndex,
} from '@entities/folder.type'
import { useUnsavedChangesWarning } from '@shared/hooks/use-unsaved-changes-warning'
import { formatCount, formatDateTime } from '@shared/lib/format'
import { Alert, AlertDescription, AlertTitle } from '@shared/ui/alert'
import { Button } from '@shared/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'
import { ScrollContainer } from '@shared/ui/scroll-container'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@shared/ui/sheet'

const FOLDERS_PATH = '/folders'
const SAVE_SHORTCUT_KEY = 's'

type FolderEditorWorkspaceProps = {
    folder: FolderDetail
    tables: FolderTableIndex[]
    onReload: () => Promise<FolderDetail | undefined>
}

type EditorDialog = 'meta' | 'import' | 'delete'

const focusAfterRender = (find: () => HTMLElement | null | undefined) => requestAnimationFrame(() => find()?.focus())

export const FolderEditorWorkspace: FC<FolderEditorWorkspaceProps> = ({ folder, tables, onReload }) => {
    const listRef = useRef<HTMLOListElement>(null)
    const menuTriggerRef = useRef<HTMLButtonElement>(null)
    const importButtonRef = useRef<HTMLButtonElement>(null)
    const noteReturnRef = useRef<HTMLElement | null>(null)

    const [base, setBase] = useState<FolderBase>(() => toFolderBase(folder))
    const [draft, setDraft] = useState(() => toFolderDraft(folder))
    const [seenFolder, setSeenFolder] = useState(folder)
    const [conflict, setConflict] = useState<FolderDetail | null>(null)
    const [saveError, setSaveError] = useState('')
    const [isSaving, setIsSaving] = useState(false)
    const [dialog, setDialog] = useState<EditorDialog | null>(null)
    const [noteTarget, setNoteTarget] = useState<FolderItem | null>(null)
    const [leaveHref, setLeaveHref] = useState<string | null>(null)
    const [isSearchSheetOpen, setIsSearchSheetOpen] = useState(false)

    const router = useRouter()
    const replaceItems = useReplaceFolderItems()
    const updateFolder = useUpdateFolder()
    const deleteFolder = useDeleteFolder()

    const changes = countFolderDraftChanges(base, draft)
    const isDirty = changes.total > 0

    if (folder !== seenFolder) {
        setSeenFolder(folder)
        if (!isDirty && !isSaving) {
            setBase(toFolderBase(folder))
            setDraft(toFolderDraft(folder))
        }
    }

    const levelLabelByMd5 = buildLevelLabelByMd5(tables)
    const storedMd5s = new Set(base.items.map((item) => item.md5))
    const presentMd5s = new Set(draft.items.map((item) => item.md5))
    const capacity = Math.max(0, FOLDER_LIMIT.ITEMS_PER_FOLDER - draft.items.length)
    const isFull = capacity === 0
    const itemCountLabel = `${formatCount(draft.items.length)}곡`
    const headerDescription = isDirty
        ? `${itemCountLabel} · 저장하지 않은 변경 ${changes.total}건`
        : `${itemCountLabel} · ${formatDateTime(base.updated_at)} 수정`

    const save = async (target: FolderBase) => {
        if (isSaving) return
        const snapshot = draft
        setIsSaving(true)
        setSaveError('')
        setConflict(null)
        try {
            const change = await replaceItems.mutateAsync({
                folderId: folder.id,
                items: toReplaceItems(target, snapshot),
                baseUpdatedAt: target.updated_at,
            })
            const savedItems = { ...target, items: snapshot.items, updated_at: change.updated_at }
            setBase(savedItems)
            const metaInput = {
                ...(snapshot.name !== target.name ? { name: snapshot.name } : {}),
                ...(snapshot.description !== target.description ? { description: snapshot.description } : {}),
            }
            if (Object.keys(metaInput).length > 0) {
                const summary = await updateFolder.mutateAsync({ folderId: folder.id, input: metaInput })
                setBase({ ...savedItems, name: summary.name, description: summary.description, updated_at: summary.updated_at })
            }
            const fresh = await onReload()
            if (fresh) {
                setBase(toFolderBase(fresh))
                setDraft((current) => (current === snapshot ? toFolderDraft(fresh) : current))
            }
            toast.success('저장했습니다. R-BMS 는 다음에 로그인할 때 바뀐 폴더를 읽습니다.')
        } catch (error) {
            if (getFolderErrorCode(error) === FOLDER_ERROR_CODE.CONFLICT) {
                const latest = await onReload()
                if (latest) setConflict(latest)
                else setSaveError(toFolderErrorMessage({ code: FOLDER_ERROR_CODE.NOT_FOUND }))
                return
            }
            const message = toFolderErrorMessage(error)
            setSaveError(message)
            toast.error(message)
        } finally {
            setIsSaving(false)
        }
    }

    const handleMove = (index: number, move: FolderChartMove) => {
        const md5 = draft.items[index]?.md5
        setDraft((current) => ({ ...current, items: moveFolderItem(current.items, index, move) }))
        if (move === 'up' || move === 'down')
            focusAfterRender(() => listRef.current?.querySelector<HTMLElement>(`[data-md5="${md5}"] [data-action="${move}"]`))
    }

    const handleRemove = (index: number) => {
        const nextLength = draft.items.length - 1
        setDraft((current) => ({ ...current, items: current.items.filter((_, position) => position !== index) }))
        focusAfterRender(() => {
            const rows = listRef.current?.querySelectorAll<HTMLElement>('li [data-action="remove"]')
            if (rows && nextLength > 0) return rows[Math.min(index, nextLength - 1)]
            return importButtonRef.current
        })
    }

    const handleAdd = (item: Pick<FolderItem, 'md5' | 'sha256' | 'title' | 'artist'>) =>
        setDraft((current) => ({ ...current, items: appendFolderItems(current.items, [item]).items }))

    const handleImport = (entries: Pick<FolderItem, 'md5' | 'title' | 'artist'>[]) => {
        const result = appendFolderItems(draft.items, entries)
        setDraft({ ...draft, items: result.items })
        setDialog(null)
        toast.success(`${formatCount(result.added)}곡을 목록 끝에 넣었습니다. 저장해야 R-BMS 에 반영됩니다.`)
    }

    const handleRevert = () => {
        setDraft(toFolderDraft(base))
        setConflict(null)
        setSaveError('')
        focusAfterRender(() => listRef.current ?? importButtonRef.current)
    }

    const handleUseServer = (server: FolderDetail) => {
        setBase(toFolderBase(server))
        setDraft(toFolderDraft(server))
        setConflict(null)
        focusAfterRender(() => listRef.current ?? importButtonRef.current)
    }

    const handleOverwrite = (server: FolderDetail) => {
        const target = toFolderBase(server)
        setBase(target)
        void save(target)
    }

    const handleDelete = () =>
        deleteFolder.mutate(folder.id, {
            onSuccess: () => {
                setDialog(null)
                setDraft(toFolderDraft(base))
                toast.success(`'${base.name}' 폴더를 지웠습니다.`)
                router.replace(FOLDERS_PATH)
            },
        })

    const handleLeave = () => {
        if (leaveHref === null) return
        const href = leaveHref
        setLeaveHref(null)
        setDraft(toFolderDraft(base))
        router.push(href)
    }

    const searchPanelProps = {
        tables,
        levelLabelByMd5,
        presentMd5s,
        isFull,
        onAdd: handleAdd,
    }

    const handleSaveShortcut = useEffectEvent((event: KeyboardEvent) => {
        if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== SAVE_SHORTCUT_KEY) return
        event.preventDefault()
        if (isDirty && dialog === null && noteTarget === null && leaveHref === null) void save(base)
    })

    useUnsavedChangesWarning(isDirty)
    useFolderLeaveGuard(isDirty, setLeaveHref)
    useEffect(() => {
        window.addEventListener('keydown', handleSaveShortcut)
        return () => window.removeEventListener('keydown', handleSaveShortcut)
    }, [])

    return (
        <section className='rbms-page'>
            <ShellPageHeader
                title={draft.name}
                description={headerDescription}
                actions={
                    <>
                        <Button ref={importButtonRef} variant='outline' size='sm' aria-label='표에서 가져오기' onClick={() => setDialog('import')}>
                            <Import />
                            <span className='hidden sm:inline'>표에서 가져오기</span>
                        </Button>
                        <Button variant='outline' size='sm' aria-label='곡 추가' className='xl:hidden' onClick={() => setIsSearchSheetOpen(true)}>
                            <Plus />
                            <span className='hidden sm:inline'>곡 추가</span>
                        </Button>
                        <Button
                            size='sm'
                            aria-disabled={!isDirty || isSaving}
                            className='aria-disabled:opacity-50'
                            onClick={() => isDirty && void save(base)}>
                            <Save />
                            {isSaving ? '저장 중' : '저장'}
                        </Button>
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <Button ref={menuTriggerRef} variant='ghost' size='icon-sm' aria-label='폴더 메뉴'>
                                    <Ellipsis />
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align='end' className='w-48'>
                                <DropdownMenuItem onSelect={() => setDialog('meta')}>
                                    <Pencil />
                                    이름·설명 고치기
                                </DropdownMenuItem>
                                <DropdownMenuItem disabled={!isDirty} onSelect={handleRevert}>
                                    <Undo2 />
                                    변경 되돌리기
                                </DropdownMenuItem>
                                <DropdownMenuSeparator />
                                <DropdownMenuItem variant='destructive' onSelect={() => setDialog('delete')}>
                                    <Trash2 />
                                    폴더 지우기
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </>
                }
            />
            <ShellSidebarPortal>
                <FolderGuidePanel />
            </ShellSidebarPortal>
            {conflict && (
                <FolderConflictNotice
                    summary={summariseFolderConflict(draft, conflict)}
                    isSaving={isSaving}
                    onUseServer={() => handleUseServer(conflict)}
                    onOverwrite={() => handleOverwrite(conflict)}
                />
            )}
            {saveError && !conflict && (
                <Alert role='alert' variant='destructive' className='rounded-none border-x-0 border-t-0'>
                    <AlertTitle>저장하지 못했습니다</AlertTitle>
                    <AlertDescription>{saveError} 변경은 이 화면에 그대로 있습니다.</AlertDescription>
                </Alert>
            )}
            <div className='flex min-w-0 flex-1 flex-col md:min-h-0 md:flex-row'>
                <div className='flex min-w-0 flex-1 flex-col md:min-h-0'>
                    <div className='border-border bg-background flex h-9 shrink-0 items-center justify-between gap-2 border-b px-3 text-xs'>
                        <p className='flex min-w-0 items-center gap-2 tabular-nums'>
                            <span>
                                {itemCountLabel}
                                <span className='text-muted-foreground'> / {formatCount(FOLDER_LIMIT.ITEMS_PER_FOLDER)}</span>
                            </span>
                            {isDirty && <span className='text-warning truncate'>저장 안 됨 · {describeFolderDraftChanges(changes)}</span>}
                        </p>
                        {isDirty && (
                            <Button variant='ghost' size='xs' onClick={handleRevert}>
                                <Undo2 />
                                되돌리기
                            </Button>
                        )}
                    </div>
                    <ScrollContainer className='pb-12'>
                        {draft.items.length === 0 ? (
                            <EmptyState
                                title='곡이 없습니다'
                                description='곡을 찾아 넣거나 발광 난이도표에서 레벨째 가져오세요. 넣은 뒤 저장해야 R-BMS 에 보입니다.'
                                className='min-h-64'
                                action={
                                    <div className='flex flex-wrap justify-center gap-1'>
                                        <Button variant='outline' size='sm' onClick={() => setDialog('import')}>
                                            <Import />
                                            표에서 가져오기
                                        </Button>
                                        <Button variant='outline' size='sm' className='xl:hidden' onClick={() => setIsSearchSheetOpen(true)}>
                                            <Plus />곡 추가
                                        </Button>
                                    </div>
                                }
                            />
                        ) : (
                            <ol ref={listRef} tabIndex={-1} aria-label={`${draft.name}의 곡`} className='flex flex-col outline-none'>
                                {draft.items.map((item, index) => (
                                    <FolderChartRow
                                        key={item.md5}
                                        item={item}
                                        position={index + 1}
                                        levelLabel={levelLabelByMd5.get(item.md5)}
                                        isNew={!storedMd5s.has(item.md5)}
                                        isFirst={index === 0}
                                        isLast={index === draft.items.length - 1}
                                        onMove={(move) => handleMove(index, move)}
                                        onEditNote={() => {
                                            noteReturnRef.current =
                                                listRef.current?.querySelector<HTMLElement>(`[data-md5="${item.md5}"] [data-action="menu"]`) ?? null
                                            setNoteTarget(item)
                                        }}
                                        onRemove={() => handleRemove(index)}
                                    />
                                ))}
                            </ol>
                        )}
                    </ScrollContainer>
                </div>
                <aside aria-label='곡 추가' className='border-border bg-sidebar w-context-panel hidden shrink-0 flex-col border-l xl:flex xl:min-h-0'>
                    <FolderChartSearchPanel {...searchPanelProps} className='flex-1' />
                </aside>
            </div>
            <Sheet open={isSearchSheetOpen} onOpenChange={setIsSearchSheetOpen}>
                <SheetContent side='right' className='gap-0 p-0 data-[side=right]:w-full data-[side=right]:sm:max-w-sm'>
                    <SheetHeader className='border-border border-b pr-12'>
                        <SheetTitle>곡 추가</SheetTitle>
                        <SheetDescription>넣은 곡은 목록 끝에 붙고, 저장해야 R-BMS 에 반영됩니다.</SheetDescription>
                    </SheetHeader>
                    <FolderChartSearchPanel {...searchPanelProps} autoFocus isLabelVisible={false} className='flex-1' />
                </SheetContent>
            </Sheet>
            <FolderMetaDialog
                open={dialog === 'meta'}
                onOpenChange={(open) => setDialog(open ? 'meta' : null)}
                title='이름·설명 고치기'
                description='저장 버튼을 눌러야 반영됩니다.'
                submitLabel='적용'
                initialValue={{ name: draft.name, description: draft.description }}
                onSubmit={(value) => {
                    setDraft((current) => ({ ...current, ...value }))
                    setDialog(null)
                }}
                returnFocusRef={menuTriggerRef}
            />
            <FolderTableImportDialog
                open={dialog === 'import'}
                onOpenChange={(open) => setDialog(open ? 'import' : null)}
                tables={tables}
                existingMd5s={presentMd5s}
                capacity={capacity}
                onImport={handleImport}
                returnFocusRef={importButtonRef}
            />
            <FolderNoteDialog
                open={noteTarget !== null}
                onOpenChange={(open) => !open && setNoteTarget(null)}
                chartTitle={noteTarget?.title ?? ''}
                initialNote={noteTarget?.note ?? ''}
                onSubmit={(note) => {
                    const md5 = noteTarget?.md5
                    setDraft((current) => ({ ...current, items: current.items.map((item) => (item.md5 === md5 ? { ...item, note } : item)) }))
                    setNoteTarget(null)
                }}
                returnFocusRef={noteReturnRef}
            />
            <ConfirmAction
                open={dialog === 'delete'}
                onOpenChange={(open) => setDialog(open ? 'delete' : null)}
                title='폴더를 지울까요?'
                description={`'${base.name}' 폴더와 그 안의 ${formatCount(base.items.length)}곡 목록을 지웁니다. R-BMS 선곡 화면에서도 사라지며 되돌릴 수 없습니다.`}
                confirmLabel='지우기'
                confirmDisabled={deleteFolder.isPending}
                onConfirm={handleDelete}
                returnFocusRef={menuTriggerRef}
            />
            <ConfirmAction
                open={leaveHref !== null}
                onOpenChange={(open) => !open && setLeaveHref(null)}
                title='저장하지 않은 변경이 있습니다'
                description={`이대로 나가면 변경 ${changes.total}건(${describeFolderDraftChanges(changes)})이 사라집니다.`}
                confirmLabel='저장하지 않고 나가기'
                cancelLabel='계속 편집'
                onConfirm={handleLeave}
            />
        </section>
    )
}
