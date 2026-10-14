'use client'

import { useState, type FC, type RefObject } from 'react'
import { FormDialog } from '@features/dialog/form-dialog'
import type { FolderTableEntry, FolderTableIndex } from '@entities/folder.type'
import { formatCount } from '@shared/lib/format'
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
            ? '가져올 레벨을 고르세요.'
            : `${formatCount(freshEntries.length)}곡 추가${duplicateCount > 0 ? ` (이미 든 ${formatCount(duplicateCount)}곡 제외)` : ''}`

    return (
        <FormDialog
            open={open}
            onOpenChange={onOpenChange}
            title='표에서 가져오기'
            description='고른 레벨의 곡을 목록 끝에 붙입니다. 저장 버튼을 눌러야 R-BMS 에 반영됩니다.'
            submitLabel={freshEntries.length > 0 ? `${formatCount(freshEntries.length)}곡 넣기` : '넣기'}
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
                    <SelectTrigger aria-label='난이도표' className='w-full'>
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
                            {table.name} 레벨
                        </FieldLegend>
                        <Button
                            type='button'
                            variant='ghost'
                            size='xs'
                            onClick={() => setSelectedLevels(isAllSelected ? [] : table.levels.map((group) => group.level))}>
                            {isAllSelected ? '모두 해제' : '모두 선택'}
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
                {overflow > 0 && (
                    <p className='text-destructive text-xs'>
                        폴더에는 {formatCount(capacity)}곡을 더 넣을 수 있습니다. 레벨을 줄여 {formatCount(overflow)}곡을 덜어 주세요.
                    </p>
                )}
            </div>
        </FormDialog>
    )
}
