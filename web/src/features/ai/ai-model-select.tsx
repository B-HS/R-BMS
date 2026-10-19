'use client'

import type { FC } from 'react'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import type { AiModel } from '@entities/ai.type'

type AiModelSelectProps = {
    models: readonly AiModel[]
    value: string | null
    disabled?: boolean
    onValueChange: (model: string) => void
}

const UNVERIFIED_SUFFIX = ' · 형식 지원 확인 안 됨'
const UNSUPPORTED_SUFFIX = ' · 형식 미지원'

const toSuffix = (structured: AiModel['structured']) => {
    if (structured === null) return UNVERIFIED_SUFFIX
    return structured ? '' : UNSUPPORTED_SUFFIX
}

export const AiModelSelect: FC<AiModelSelectProps> = ({ models, value, disabled, onValueChange }) => {
    const options =
        value !== null && !models.some((model) => model.id === value) ? [{ id: value, label: value, structured: null }, ...models] : models

    return (
        <Select value={value ?? ''} onValueChange={onValueChange} disabled={disabled}>
            <SelectTrigger aria-label='모델' size='sm' className='w-80 max-w-full'>
                <SelectValue placeholder='모델을 고르세요' />
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
