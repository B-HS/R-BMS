'use client'

import { useState, type FC, type RefObject } from 'react'
import { useTranslations } from 'next-intl'
import { FormDialog } from '@features/dialog/form-dialog'
import { FOLDER_LIMIT, folderNoteFormSchema, isFolderFormError } from '@entities/folder.type'
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
    const t = useTranslations('pages.folder')
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

    const errorText = isFolderFormError(error)
        ? t(`formError.${error}`, { nameMax: FOLDER_LIMIT.NAME_MAX, descriptionMax: FOLDER_LIMIT.DESCRIPTION_MAX, noteMax: FOLDER_LIMIT.NOTE_MAX })
        : error

    return (
        <FormDialog
            open={open}
            onOpenChange={onOpenChange}
            title={t('note.title')}
            description={chartTitle}
            submitLabel={t('note.apply')}
            onSubmit={handleSubmit}
            size='md'
            returnFocusRef={returnFocusRef}>
            <Field data-invalid={error ? true : undefined}>
                <FieldLabel htmlFor='folder-item-note'>{t('note.label')}</FieldLabel>
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
                    <FieldError id='folder-item-note-hint'>{errorText}</FieldError>
                ) : (
                    <FieldDescription id='folder-item-note-hint'>{t('note.hint', { max: FOLDER_LIMIT.NOTE_MAX })}</FieldDescription>
                )}
            </Field>
        </FormDialog>
    )
}
