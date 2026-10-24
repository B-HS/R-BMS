'use client'

import { useState, type FC, type FormEvent } from 'react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Input } from '@shared/ui/input'
import { AI_FORM_LIMIT } from '@entities/ai.type'

type AiModelIdFormProps = {
    inputId: string
    label: string
    isPending: boolean
    onSubmitModel: (model: string) => void
}

export const AiModelIdForm: FC<AiModelIdFormProps> = ({ inputId, label, isPending, onSubmitModel }) => {
    const t = useTranslations('pages.ai.modelIdForm')
    const [modelId, setModelId] = useState('')

    const trimmedModelId = modelId.trim()

    const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault()
        if (trimmedModelId !== '') onSubmitModel(trimmedModelId)
    }

    return (
        <form className='flex flex-col gap-2' onSubmit={handleSubmit}>
            <Field>
                <FieldLabel htmlFor={inputId}>{label}</FieldLabel>
                <Input
                    id={inputId}
                    value={modelId}
                    autoComplete='off'
                    spellCheck={false}
                    maxLength={AI_FORM_LIMIT.MODEL_MAX_LENGTH}
                    disabled={isPending}
                    onChange={(event) => setModelId(event.target.value)}
                    className='w-80 max-w-full font-mono'
                />
                <FieldDescription>{t('description')}</FieldDescription>
            </Field>
            <div>
                <Button type='submit' size='sm' variant='outline' disabled={isPending || trimmedModelId === ''}>
                    {t('submit')}
                </Button>
            </div>
        </form>
    )
}
