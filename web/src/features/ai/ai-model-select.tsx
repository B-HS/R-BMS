'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import type { AiModel } from '@entities/ai.type'

type AiModelSelectProps = {
    models: readonly AiModel[]
    value: string | null
    disabled?: boolean
    onValueChange: (model: string) => void
}

const SUFFIX_SEPARATOR = ' · '

export const AiModelSelect: FC<AiModelSelectProps> = ({ models, value, disabled, onValueChange }) => {
    const t = useTranslations('pages.ai.modelSelect')
    const toSuffix = (structured: AiModel['structured']) => {
        if (structured === null) return `${SUFFIX_SEPARATOR}${t('unverified')}`
        return structured ? '' : `${SUFFIX_SEPARATOR}${t('unsupported')}`
    }
    const options =
        value !== null && !models.some((model) => model.id === value) ? [{ id: value, label: value, structured: null }, ...models] : models

    return (
        <Select value={value ?? ''} onValueChange={onValueChange} disabled={disabled}>
            <SelectTrigger aria-label={t('label')} size='sm' className='w-80 max-w-full'>
                <SelectValue placeholder={t('placeholder')} />
            </SelectTrigger>
            <SelectContent>
                {options.map((model) => (
                    <SelectItem key={model.id} value={model.id} disabled={model.structured === false}>
                        {model.label}
                        {toSuffix(model.structured)}
                    </SelectItem>
                ))}
            </SelectContent>
        </Select>
    )
}
