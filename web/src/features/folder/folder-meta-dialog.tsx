'use client'

import { useState, type FC, type RefObject } from 'react'
import { useTranslations } from 'next-intl'
import { FormDialog } from '@features/dialog/form-dialog'
import { FOLDER_LIMIT, folderMetaFormSchema, isFolderFormError, type FolderMetaInput } from '@entities/folder.type'
import { Field, FieldDescription, FieldError, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { Textarea } from '@shared/ui/textarea'

type FolderMetaDialogProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    title: string
    description?: string
    submitLabel: string
    initialValue: FolderMetaInput
    isPending?: boolean
    serverError?: string
    onSubmit: (value: FolderMetaInput) => void
    returnFocusRef?: RefObject<HTMLElement | null>
}

type FieldErrors = Partial<Record<keyof FolderMetaInput, string>>

const DESCRIPTION_ROWS = 2

export const FolderMetaDialog: FC<FolderMetaDialogProps> = ({
    open,
    onOpenChange,
    title,
    description,
    submitLabel,
    initialValue,
    isPending = false,
    serverError,
    onSubmit,
    returnFocusRef,
}) => {
    const t = useTranslations('pages.folder')
    const [wasOpen, setWasOpen] = useState(open)
    const [name, setName] = useState(initialValue.name)
    const [folderDescription, setFolderDescription] = useState(initialValue.description)
    const [errors, setErrors] = useState<FieldErrors>({})

    if (open !== wasOpen) {
        setWasOpen(open)
        if (open) {
            setName(initialValue.name)
            setFolderDescription(initialValue.description)
            setErrors({})
        }
    }

    const handleSubmit = () => {
        const parsed = folderMetaFormSchema.safeParse({ name, description: folderDescription })
        if (!parsed.success) {
            setErrors(Object.fromEntries(parsed.error.issues.map((issue) => [String(issue.path[0]), issue.message])))
            return
        }
        setErrors({})
        onSubmit(parsed.data)
    }

    const describeError = (code: string | undefined) =>
        code !== undefined && isFolderFormError(code)
            ? t(`formError.${code}`, { nameMax: FOLDER_LIMIT.NAME_MAX, descriptionMax: FOLDER_LIMIT.DESCRIPTION_MAX, noteMax: FOLDER_LIMIT.NOTE_MAX })
            : code
    const nameError = describeError(errors.name)
    const descriptionError = describeError(errors.description)

    return (
        <FormDialog
            open={open}
            onOpenChange={onOpenChange}
            title={title}
            description={description}
            submitLabel={submitLabel}
            submitDisabled={isPending}
            onSubmit={handleSubmit}
            size='md'
            returnFocusRef={returnFocusRef}>
            <Field data-invalid={nameError ? true : undefined}>
                <FieldLabel htmlFor='folder-name'>{t('meta.name')}</FieldLabel>
                <Input
                    id='folder-name'
                    autoComplete='off'
                    maxLength={FOLDER_LIMIT.NAME_MAX}
                    aria-invalid={nameError ? true : undefined}
                    aria-describedby='folder-name-hint'
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                />
                {nameError ? (
                    <FieldError id='folder-name-hint'>{nameError}</FieldError>
                ) : (
                    <FieldDescription id='folder-name-hint'>{t('meta.nameHint', { max: FOLDER_LIMIT.NAME_MAX })}</FieldDescription>
                )}
            </Field>
            <Field data-invalid={descriptionError ? true : undefined}>
                <FieldLabel htmlFor='folder-description'>{t('meta.description')}</FieldLabel>
                <Textarea
                    id='folder-description'
                    rows={DESCRIPTION_ROWS}
                    maxLength={FOLDER_LIMIT.DESCRIPTION_MAX}
                    aria-invalid={descriptionError ? true : undefined}
                    aria-describedby='folder-description-hint'
                    value={folderDescription}
                    onChange={(event) => setFolderDescription(event.target.value.replace(/[\r\n]+/g, ' '))}
                    onKeyDown={(event) => {
                        if (event.key !== 'Enter' || event.shiftKey || event.nativeEvent.isComposing) return
                        event.preventDefault()
                        event.currentTarget.form?.requestSubmit()
                    }}
                />
                {descriptionError ? (
                    <FieldError id='folder-description-hint'>{descriptionError}</FieldError>
                ) : (
                    <FieldDescription id='folder-description-hint'>
                        {t('meta.descriptionHint', { max: FOLDER_LIMIT.DESCRIPTION_MAX, length: folderDescription.length })}
                    </FieldDescription>
                )}
            </Field>
            {serverError && (
                <p role='alert' className='text-destructive text-sm'>
                    {serverError}
                </p>
            )}
        </FormDialog>
    )
}
