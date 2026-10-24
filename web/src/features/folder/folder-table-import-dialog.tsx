'use client'

import { useState, type FC, type RefObject } from 'react'
import { useTranslations } from 'next-intl'
import { FormDialog } from '@features/dialog/form-dialog'
import type { FolderTableEntry, FolderTableIndex } from '@entities/folder.type'
import { Button } from '@shared/ui/button'
import { Checkbox } from '@shared/ui/checkbox'
import { FieldLegend, FieldSet } from '@shared/ui/field'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'

type FolderTableImportDialogProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    tables: FolderTableIndex[]
    existingMd5s: ReadonlySet<string>
    capacity: number
    onImport: (entries: FolderTableEntry[]) => void
    returnFocusRef?: RefObject<HTMLElement | null>
}

export const FolderTableImportDialog: FC<FolderTableImportDialogProps> = ({
    open,
    onOpenChange,
    tables,
    existingMd5s,
    capacity,
    onImport,
    returnFocusRef,
}) => {
    const t = useTranslations('pages.folder.tableImport')
    const [wasOpen, setWasOpen] = useState(open)
    const [tableId, setTableId] = useState(tables[0]?.id ?? '')
    const [selectedLevels, setSelectedLevels] = useState<string[]>([])

    if (open !== wasOpen) {
        setWasOpen(open)
        if (open) setSelectedLevels([])
    }

    const table = tables.find((candidate) => candidate.id === tableId) ?? tables[0]
    const selectedEntries = (table?.levels ?? []).filter((group) => selectedLevels.includes(group.level)).flatMap((group) => group.entries)
    const freshEntries = [...new Map(selectedEntries.filter((entry) => !existingMd5s.has(entry.md5)).map((entry) => [entry.md5, entry])).values()]
    const duplicateCount = selectedEntries.length - freshEntries.length
    const overflow = freshEntries.length - capacity
    const isAllSelected = table !== undefined && selectedLevels.length === table.levels.length

    const toggleLevel = (level: string, checked: boolean) =>
        setSelectedLevels((current) => (checked ? [...current, level] : current.filter((value) => value !== level)))

    const summary =
        selectedLevels.length === 0
            ? t('chooseLevels')
            : `${t('summaryAdd', { count: freshEntries.length })}${duplicateCount > 0 ? ` ${t('summaryDuplicates', { count: duplicateCount })}` : ''}`

    return (
        <FormDialog
            open={open}
            onOpenChange={onOpenChange}
            title={t('title')}
            description={t('description')}
            submitLabel={freshEntries.length > 0 ? t('submitCount', { count: freshEntries.length }) : t('submit')}
            submitDisabled={freshEntries.length === 0 || overflow > 0}
            onSubmit={() => onImport(freshEntries)}
            returnFocusRef={returnFocusRef}>
            {tables.length > 1 && (
                <Select
                    value={table?.id}
                    onValueChange={(value) => {
                        setTableId(value)
                        setSelectedLevels([])
                    }}>
                    <SelectTrigger aria-label={t('table')} className='w-full'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {tables.map((candidate) => (
                            <SelectItem key={candidate.id} value={candidate.id}>
                                {candidate.name}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            )}
            {table && (
                <FieldSet className='gap-2'>
                    <div className='flex items-center justify-between gap-2'>
                        <FieldLegend variant='label' className='mb-0'>
                            {t('levels', { name: table.name })}
                        </FieldLegend>
                        <Button
                            type='button'
                            variant='ghost'
                            size='xs'
                            onClick={() => setSelectedLevels(isAllSelected ? [] : table.levels.map((group) => group.level))}>
                            {isAllSelected ? t('clearAll') : t('selectAll')}
                        </Button>
                    </div>
                    <ul className='grid grid-cols-3 gap-1 sm:grid-cols-5'>
                        {table.levels.map((group) => {
                            const inputId = `import-level-${table.id}-${group.level}`
                            const isChecked = selectedLevels.includes(group.level)
                            return (
                                <li key={group.level}>
                                    <label
                                        htmlFor={inputId}
                                        data-checked={isChecked ? true : undefined}
                                        className='border-border hover:bg-muted/50 data-checked:border-foreground/60 data-checked:bg-muted flex h-9 cursor-pointer items-center gap-2 border px-2 text-xs'>
                                        <Checkbox
                                            id={inputId}
                                            checked={isChecked}
                                            onCheckedChange={(checked) => toggleLevel(group.level, checked === true)}
                                        />
                                        <span className='font-mono font-semibold'>
                                            {table.symbol}
                                            {group.level}
                                        </span>
                                        <span className='text-muted-foreground ml-auto tabular-nums'>{group.entries.length}</span>
                                    </label>
                                </li>
                            )
                        })}
                    </ul>
                </FieldSet>
            )}
            <div aria-live='polite' className='flex flex-col gap-1 text-sm'>
                <p className='tabular-nums'>{summary}</p>
                {overflow > 0 && <p className='text-destructive text-xs'>{t('overflow', { capacity, overflow })}</p>}
            </div>
        </FormDialog>
    )
}
