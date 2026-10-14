'use client'

import { useState, type FC, type RefObject } from 'react'
import { FormDialog } from '@features/dialog/form-dialog'
import { FOLDER_LIMIT, folderNoteFormSchema } from '@entities/folder.type'
import { Field, FieldDescription, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'

type FolderNoteDialogProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    chartTitle: string
    initialNote: string
    onSubmit: (note: string) => void
    returnFocusRef?: RefObject<HTMLElement | null>
}

export const FolderNoteDialog: FC<FolderNoteDialogProps> = ({ open, onOpenChange, chartTitle, initialNote, onSubmit, returnFocusRef }) => {
    const [wasOpen, setWasOpen] = useState(open)
    const [note, setNote] = useState(initialNote)
    const [error, setError] = useState('')

    if (open !== wasOpen) {
        setWasOpen(open)
        if (open) {
            setNote(initialNote)
            setError('')
        }
    }

    const handleSubmit = () => {
        const parsed = folderNoteFormSchema.safeParse(note)
        if (!parsed.success) {
            setError(parsed.error.issues[0]?.message ?? '')
            return
        }
        onSubmit(parsed.data)
    }

    return (
        <FormDialog
            open={open}
            onOpenChange={onOpenChange}
            title='메모 고치기'
            description={chartTitle}
            submitLabel='적용'
            onSubmit={handleSubmit}
            size='md'
            returnFocusRef={returnFocusRef}>
            <Field data-invalid={error ? true : undefined}>
                <FieldLabel htmlFor='folder-item-note'>메모</FieldLabel>
                <Input
                    id='folder-item-note'
                    autoComplete='off'
                    maxLength={FOLDER_LIMIT.NOTE_MAX}
                    aria-invalid={error ? true : undefined}
                    aria-describedby='folder-item-note-hint'
                    value={note}
                    onChange={(event) => setNote(event.target.value)}
                />
                {error ? (
                    <FieldError id='folder-item-note-hint'>{error}</FieldError>
                ) : (
                    <FieldDescription id='folder-item-note-hint'>{FOLDER_LIMIT.NOTE_MAX}자까지. 저장 버튼을 눌러야 반영됩니다.</FieldDescription>
                )}
            </Field>
        </FormDialog>
    )
}
