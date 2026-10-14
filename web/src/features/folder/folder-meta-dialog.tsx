'use client'

import { useState, type FC, type RefObject } from 'react'
import { FormDialog } from '@features/dialog/form-dialog'
import { FOLDER_LIMIT, folderMetaFormSchema, type FolderMetaInput } from '@entities/folder.type'
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

    const nameError = errors.name

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
                <FieldLabel htmlFor='folder-name'>이름</FieldLabel>
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
                    <FieldDescription id='folder-name-hint'>
                        R-BMS 선곡 화면에 보이는 이름입니다. {FOLDER_LIMIT.NAME_MAX}자까지, 다른 폴더와 겹치면 안 됩니다.
                    </FieldDescription>
                )}
            </Field>
            <Field data-invalid={errors.description ? true : undefined}>
                <FieldLabel htmlFor='folder-description'>설명 (선택)</FieldLabel>
                <Textarea
                    id='folder-description'
                    rows={DESCRIPTION_ROWS}
                    maxLength={FOLDER_LIMIT.DESCRIPTION_MAX}
                    aria-invalid={errors.description ? true : undefined}
                    aria-describedby='folder-description-hint'
                    value={folderDescription}
                    onChange={(event) => setFolderDescription(event.target.value.replace(/[\r\n]+/g, ' '))}
                    onKeyDown={(event) => {
                        if (event.key !== 'Enter' || event.shiftKey || event.nativeEvent.isComposing) return
                        event.preventDefault()
                        event.currentTarget.form?.requestSubmit()
                    }}
                />
                {errors.description ? (
                    <FieldError id='folder-description-hint'>{errors.description}</FieldError>
                ) : (
                    <FieldDescription id='folder-description-hint'>
                        한 줄, {FOLDER_LIMIT.DESCRIPTION_MAX}자까지. {folderDescription.length} / {FOLDER_LIMIT.DESCRIPTION_MAX}
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
