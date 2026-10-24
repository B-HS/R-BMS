'use client'

import { useRef, type FC, type FormEvent } from 'react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { AI_FORM_LIMIT } from '@entities/ai.type'

type AiKeyFormProps = {
    inputId: string
    label: string
    submitLabel: string
    isPending: boolean
    onSubmitKey: (apiKey: string) => void
    onCancel?: () => void
}

export const AiKeyForm: FC<AiKeyFormProps> = ({ inputId, label, submitLabel, isPending, onSubmitKey, onCancel }) => {
    const t = useTranslations('pages.ai')
    const tShared = useTranslations('pages.shared')
    const keyInputRef = useRef<HTMLInputElement>(null)

    const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()
        const input = keyInputRef.current
        if (!input) return
        const apiKey = input.value.trim()
        if (apiKey.length < AI_FORM_LIMIT.API_KEY_MIN_LENGTH) return
        input.value = ''
        onSubmitKey(apiKey)
    }

    return (
        <form className='flex flex-col gap-2' onSubmit={handleSubmit}>
            <Field>
                <FieldLabel htmlFor={inputId}>{label}</FieldLabel>
                <Input
                    ref={keyInputRef}
                    id={inputId}
                    type='password'
                    name='api_key'
                    autoComplete='off'
                    spellCheck={false}
                    required
                    minLength={AI_FORM_LIMIT.API_KEY_MIN_LENGTH}
                    maxLength={AI_FORM_LIMIT.API_KEY_MAX_LENGTH}
                    disabled={isPending}
                />
                <FieldDescription>{t('keyForm.description')}</FieldDescription>
            </Field>
            <div className='flex gap-1'>
                <Button type='submit' size='sm' disabled={isPending}>
                    {submitLabel}
                </Button>
                {onCancel && (
                    <Button type='button' variant='ghost' size='sm' onClick={onCancel}>
                        {tShared('cancel')}
                    </Button>
                )}
            </div>
        </form>
    )
}
