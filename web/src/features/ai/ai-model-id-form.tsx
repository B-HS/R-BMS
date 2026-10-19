'use client'

import { useState, type FC, type FormEvent } from 'react'
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
                <FieldDescription>
                    목록을 받지 못하면 공급자 콘솔에 보이는 모델 id 를 그대로 적으세요. 틀린 id 는 호출할 때 실패합니다.
                </FieldDescription>
            </Field>
            <div>
                <Button type='submit' size='sm' variant='outline' disabled={isPending || trimmedModelId === ''}>
                    이 모델 쓰기
                </Button>
            </div>
        </form>
    )
}
