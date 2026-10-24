'use client'

import { useDeferredValue, useEffect, useRef, useState, type FC } from 'react'
import Link from 'next/link'
import { usePathname, useSearchParams } from 'next/navigation'
import { ChevronsDownUp, ChevronsUpDown, Search, SlidersHorizontal } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { EmptyState } from '@features/state/empty-state'
import { TableChartGrid } from '@features/table/table-chart-grid'
import { TableDeferredMount } from '@features/table/table-deferred-mount'
import { TableFilterFields } from '@features/table/table-filter-fields'
import { TableLevelJump } from '@features/table/table-level-jump'
import { TableLevelSection } from '@features/table/table-level-section'
import { TableProgressPanel } from '@features/table/table-progress-panel'
import { TableChartDialog } from '@widgets/table/table-chart-dialog'
import { TableSnapshotSidebar } from '@widgets/table/table-snapshot-sidebar'
import {
    countActiveTableFilters,
    countPlayedEntries,
    countTableLamps,
    DEFAULT_TABLE_FILTER,
    filterTableLevels,
    levelAnchorId,
    parseTableLampFilter,
    parseTableSort,
    readTableFilter,
    TABLE_LAMP_FILTERS,
    TABLE_SORTS,
    writeTableFilter,
    type TableFilterState,
} from '@widgets/table/table-snapshot-filter'
import { useSessionUser } from '@entities/auth.query'
import { useGetTableSnapshotRecords } from '@entities/table-snapshot.query'
import type { TableSnapshotDetail, TableSnapshotEntry } from '@entities/table-snapshot.type'
import { buildLoginHref } from '@shared/constants/auth-route'
import { Alert, AlertAction, AlertDescription } from '@shared/ui/alert'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { Input } from '@shared/ui/input'
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle, SheetTrigger } from '@shared/ui/sheet'

type TableSnapshotWorkspaceProps = {
    detail: TableSnapshotDetail
    initialUserId: string | null
    tableUrl: string
}

const SEARCH_COMMIT_DELAY_MS = 250
const SEARCH_SHORTCUT_KEY = '/'
const ANY_LEVEL = 'any'
const EAGER_LEVEL_COUNT = 2

const isTypingTarget = (target: EventTarget | null) =>
    target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))

const replaceAddress = (filter: TableFilterState, mode: 'push' | 'replace') => {
    const query = writeTableFilter(filter)
    const url = `${window.location.pathname}${query ? `?${query}` : ''}${window.location.hash}`
    if (mode === 'push') window.history.pushState(null, '', url)
    else window.history.replaceState(null, '', url)
}

export const TableSnapshotWorkspace: FC<TableSnapshotWorkspaceProps> = ({ detail, initialUserId, tableUrl }) => {
    const searchParams = useSearchParams()
    const pathname = usePathname()
    const t = useTranslations('browse.table.workspace')
    const { user, isPending: isSessionPending } = useSessionUser()
    const { table, levels } = detail
    const levelOrder = levels.map((group) => group.level)
    const filter = readTableFilter(searchParams, levelOrder)

    const searchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
    const searchInputRef = useRef<HTMLInputElement>(null)
    const dialogTriggerRef = useRef<HTMLButtonElement | null>(null)

    const [searchText, setSearchText] = useState(filter.query)
    const [syncedQuery, setSyncedQuery] = useState(filter.query)
    const [collapsedLevels, setCollapsedLevels] = useState<string[]>([])
    const [selectedEntry, setSelectedEntry] = useState<TableSnapshotEntry | null>(null)
    const [isDialogOpen, setIsDialogOpen] = useState(false)
    const [isFilterSheetOpen, setIsFilterSheetOpen] = useState(false)

    if (filter.query !== syncedQuery) {
        setSyncedQuery(filter.query)
        setSearchText(filter.query)
    }

    const userId = isSessionPending ? initialUserId : (user?.id ?? null)
    const recordsQuery = useGetTableSnapshotRecords(table.id, userId)
    const records = recordsQuery.data ?? null
    const isRecordPending = userId !== null && recordsQuery.isPending
    const deferredSearchText = useDeferredValue(searchText)
    const visibleLevels = filterTableLevels(levels, { ...filter, query: deferredSearchText }, records)
    const shownCount = visibleLevels.reduce((total, group) => total + group.entries.length, 0)
    const visibleLevelNames = new Set(visibleLevels.map((group) => group.level))
    const isEveryVisibleCollapsed = visibleLevels.length > 0 && visibleLevels.every((group) => collapsedLevels.includes(group.level))
    const activeFilterCount = countActiveTableFilters(filter)
    const hasNarrowing = activeFilterCount > 0 || searchText.trim() !== ''
    const loginHref = buildLoginHref(`${pathname}${searchParams.size > 0 ? `?${searchParams.toString()}` : ''}`)
    const levelLabel = (level: string) => `${table.symbol}${level}`
    const levelOptions = [{ value: ANY_LEVEL, label: t('all') }, ...levelOrder.map((level) => ({ value: level, label: levelLabel(level) }))]
    const sortOptions = TABLE_SORTS.map((sort) => ({
        value: sort.value,
        label: t(`sorts.${sort.value}`),
        disabled: sort.needsRecords && records === null,
    }))
    const allEntries = levels.flatMap((group) => group.entries)
    const overallLamps = records ? countTableLamps(allEntries, records) : null
    const overallPlayed = records ? countPlayedEntries(allEntries, records) : 0

    const commitFilter = (next: Partial<TableFilterState>) => {
        if (searchTimerRef.current !== null) clearTimeout(searchTimerRef.current)
        replaceAddress({ ...filter, query: searchText, ...next }, 'push')
    }

    const handleSearchChange = (value: string) => {
        setSearchText(value)
        if (searchTimerRef.current !== null) clearTimeout(searchTimerRef.current)
        searchTimerRef.current = setTimeout(
            () => replaceAddress({ ...readTableFilter(new URLSearchParams(window.location.search), levelOrder), query: value }, 'replace'),
            SEARCH_COMMIT_DELAY_MS,
        )
    }

    const handleReset = () => {
        setSearchText('')
        commitFilter(DEFAULT_TABLE_FILTER)
    }

    const handleRangeChange = (edge: 'from' | 'to', value: string) => {
        const next = { from: filter.from, to: filter.to, [edge]: value === ANY_LEVEL ? null : value }
        const isReversed = next.from !== null && next.to !== null && levelOrder.indexOf(next.from) > levelOrder.indexOf(next.to)
        commitFilter(isReversed ? { from: next.to, to: next.from } : next)
    }

    const handleJump = (anchorId: string) =>
        setCollapsedLevels((current) => current.filter((level) => levelAnchorId(level, levelOrder.indexOf(level)) !== anchorId))

    const handleOpenEntry = (entry: TableSnapshotEntry, trigger: HTMLButtonElement) => {
        dialogTriggerRef.current = trigger
        setSelectedEntry(entry)
        setIsDialogOpen(true)
    }

    const filterFields = (layout: 'inline' | 'stacked') => (
        <TableFilterFields
            layout={layout}
            levelOptions={levelOptions}
            from={filter.from ?? ANY_LEVEL}
            to={filter.to ?? ANY_LEVEL}
            onFromChange={(value) => handleRangeChange('from', value)}
            onToChange={(value) => handleRangeChange('to', value)}
            lampOptions={TABLE_LAMP_FILTERS.map((option) => ({ value: option.value, label: t(`lampFilters.${option.value}`) }))}
            lamp={records ? filter.lamp : DEFAULT_TABLE_FILTER.lamp}
            isLampDisabled={records === null}
            lampHint={userId === null ? t('signedOutLampHint') : undefined}
            onLampChange={(value) => commitFilter({ lamp: parseTableLampFilter(value) })}
            sortOptions={sortOptions}
            sort={sortOptions.find((option) => option.value === filter.sort)?.disabled ? DEFAULT_TABLE_FILTER.sort : filter.sort}
            onSortChange={(value) => commitFilter({ sort: parseTableSort(value) })}
        />
    )

    useEffect(() => {
        const anchorId = window.location.hash.slice(1)
        if (anchorId) document.getElementById(anchorId)?.scrollIntoView({ block: 'start', behavior: 'instant' })
    }, [])

    useEffect(() => {
        const handleShortcut = (event: KeyboardEvent) => {
            if (event.key !== SEARCH_SHORTCUT_KEY || event.metaKey || event.ctrlKey || event.altKey || isTypingTarget(event.target)) return
            event.preventDefault()
            searchInputRef.current?.focus()
        }
        window.addEventListener('keydown', handleShortcut)
        return () => {
            window.removeEventListener('keydown', handleShortcut)
            if (searchTimerRef.current !== null) clearTimeout(searchTimerRef.current)
        }
    }, [])

    return (
        <>
            {overallLamps && (
                <TableProgressPanel
                    lamps={overallLamps}
                    played={overallPlayed}
                    total={table.entry_count}
                    levels={levels.map((group) => ({
                        key: group.level,
                        label: levelLabel(group.level),
                        lamps: countTableLamps(group.entries, records ?? {}),
                        played: countPlayedEntries(group.entries, records ?? {}),
                        total: group.entries.length,
                    }))}
                />
            )}
            {userId === null && (
                <div className='bg-card flex flex-wrap items-center justify-between gap-2 px-3 py-2'>
                    <p className='text-muted-foreground text-xs'>{t('loginPrompt')}</p>
                    <Button asChild variant='outline' size='sm'>
                        <Link href={loginHref}>{t('signIn')}</Link>
                    </Button>
                </div>
            )}
            {recordsQuery.isError && (
                <Alert className='bg-card rounded-none border-0'>
                    <AlertDescription className='text-xs'>{t('recordsError')}</AlertDescription>
                    <AlertAction>
                        <Button variant='outline' size='xs' disabled={recordsQuery.isFetching} onClick={() => void recordsQuery.refetch()}>
                            {t('retry')}
                        </Button>
                    </AlertAction>
                </Alert>
            )}

            <div role='search' aria-label={t('searchLabel')} className='bg-background flex min-h-12 flex-wrap items-center gap-2 border-b px-3 py-2'>
                <div className='relative min-w-0 flex-1 md:w-56 md:flex-none'>
                    <Search aria-hidden className='text-muted-foreground pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2' />
                    <Input
                        ref={searchInputRef}
                        type='search'
                        value={searchText}
                        onChange={(event) => handleSearchChange(event.target.value)}
                        placeholder={t('searchPlaceholder')}
                        aria-label={t('searchAria')}
                        aria-keyshortcuts={SEARCH_SHORTCUT_KEY}
                        className='h-8 pl-7 text-sm md:h-7 md:text-xs'
                    />
                </div>
                <div className='hidden md:block'>{filterFields('inline')}</div>
                <Sheet open={isFilterSheetOpen} onOpenChange={setIsFilterSheetOpen}>
                    <SheetTrigger asChild>
                        <Button type='button' variant='outline' size='sm' className='relative md:hidden'>
                            <SlidersHorizontal aria-hidden />
                            {t('filter')}
                            {activeFilterCount > 0 && (
                                <Badge className='text-2xs h-4 min-w-4 rounded-none px-1 tabular-nums'>
                                    <span aria-hidden>{activeFilterCount}</span>
                                    <span className='sr-only'>{t('filtersApplied', { count: activeFilterCount })}</span>
                                </Badge>
                            )}
                        </Button>
                    </SheetTrigger>
                    <SheetContent side='bottom' className='max-h-[85dvh] gap-0 rounded-none'>
                        <SheetHeader className='border-b'>
                            <SheetTitle>{t('filter')}</SheetTitle>
                            <SheetDescription className='text-xs'>{t('filterDescription')}</SheetDescription>
                        </SheetHeader>
                        <div className='overflow-y-auto p-4'>{filterFields('stacked')}</div>
                        <SheetFooter className='flex-row items-center justify-between border-t'>
                            <span className='text-muted-foreground text-xs tabular-nums'>
                                {t('shownOfTotal', { shown: shownCount, total: table.entry_count })}
                            </span>
                            <div className='flex gap-2'>
                                <Button type='button' variant='outline' size='sm' disabled={!hasNarrowing} onClick={handleReset}>
                                    {t('reset')}
                                </Button>
                                <Button type='button' size='sm' onClick={() => setIsFilterSheetOpen(false)}>
                                    {t('showResults')}
                                </Button>
                            </div>
                        </SheetFooter>
                    </SheetContent>
                </Sheet>
                <div className='ml-auto flex shrink-0 items-center gap-1'>
                    <span role='status' className='text-muted-foreground px-1 text-xs tabular-nums'>
                        {hasNarrowing
                            ? t('shownOfTotal', { shown: shownCount, total: table.entry_count })
                            : t('chartCount', { count: table.entry_count })}
                    </span>
                    {hasNarrowing && (
                        <Button type='button' variant='ghost' size='sm' onClick={handleReset} className='hidden md:inline-flex'>
                            {t('reset')}
                        </Button>
                    )}
                    <Button
                        type='button'
                        variant='ghost'
                        size='icon-sm'
                        aria-label={isEveryVisibleCollapsed ? t('expandAll') : t('collapseAll')}
                        disabled={visibleLevels.length === 0}
                        onClick={() => setCollapsedLevels(isEveryVisibleCollapsed ? [] : levelOrder)}>
                        {isEveryVisibleCollapsed ? <ChevronsUpDown aria-hidden /> : <ChevronsDownUp aria-hidden />}
                    </Button>
                </div>
            </div>

            <TableLevelJump
                items={levels.map((group, index) => ({
                    anchorId: levelAnchorId(group.level, index),
                    label: levelLabel(group.level),
                    isAvailable: visibleLevelNames.has(group.level),
                }))}
                onJump={handleJump}
                className='border-b'
            />

            {visibleLevels.length === 0 && (
                <EmptyState
                    title={t('emptyTitle')}
                    description={t('emptyDescription')}
                    action={
                        <Button variant='outline' size='sm' onClick={handleReset}>
                            {t('resetFilters')}
                        </Button>
                    }
                />
            )}
            {visibleLevels.map((group, index) => (
                <TableLevelSection
                    key={group.level}
                    anchorId={group.anchorId}
                    label={levelLabel(group.level)}
                    total={group.levelEntries.length}
                    shown={group.entries.length}
                    lamps={records ? countTableLamps(group.levelEntries, records) : null}
                    played={records ? countPlayedEntries(group.levelEntries, records) : null}
                    isRecordPending={isRecordPending}
                    isOpen={!collapsedLevels.includes(group.level)}
                    onOpenChange={(isOpen) =>
                        setCollapsedLevels((current) => (isOpen ? current.filter((level) => level !== group.level) : [...current, group.level]))
                    }>
                    <TableDeferredMount itemCount={group.entries.length} isInitiallyMounted={index < EAGER_LEVEL_COUNT}>
                        <TableChartGrid
                            label={t('gridLabel', { level: levelLabel(group.level) })}
                            symbol={table.symbol}
                            entries={group.entries}
                            records={records}
                            isRecordPending={isRecordPending}
                            onOpen={handleOpenEntry}
                        />
                    </TableDeferredMount>
                </TableLevelSection>
            ))}

            <TableSnapshotSidebar table={table} tableUrl={tableUrl} />
            <TableChartDialog
                entry={selectedEntry}
                isOpen={isDialogOpen}
                onOpenChange={setIsDialogOpen}
                onCloseAutoFocus={(event) => {
                    if (!dialogTriggerRef.current?.isConnected) return
                    event.preventDefault()
                    dialogTriggerRef.current.focus()
                }}
                symbol={table.symbol}
                record={selectedEntry && records ? records[selectedEntry.md5] : undefined}
                isSignedIn={userId !== null}
                isRecordPending={isRecordPending}
                loginHref={loginHref}
            />
        </>
    )
}
