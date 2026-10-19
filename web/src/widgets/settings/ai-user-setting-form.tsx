'use client'

import { useState, type FC } from 'react'
import { toast } from 'sonner'
import { AiLimitNumberField } from '@features/ai/ai-limit-number-field'
import { parseLimitInput } from '@features/ai/parse-limit-input'
import { Button } from '@shared/ui/button'
import { Field, FieldDescription, FieldLabel } from '@shared/ui/field'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@shared/ui/select'
import { formatCount } from '@shared/lib/format'
import { useSaveAiUserSetting } from '@entities/ai.query'
import { AI_FORM_LIMIT, AI_PROVIDER_LABEL, type AiCredential, type AiUserSetting } from '@entities/ai.type'

const AUTO_PROVIDER = 'auto'

type AiUserSettingFormProps = {
    setting: AiUserSetting
    credentials: readonly AiCredential[]
}

export const AiUserSettingForm: FC<AiUserSettingFormProps> = ({ setting, credentials }) => {
    const [providerDraft, setProviderDraft] = useState<string | null>(null)
    const [limitDraft, setLimitDraft] = useState<string | null>(null)

    const save = useSaveAiUserSetting()

    const savedProvider = credentials.find((credential) => credential.provider === setting.default_provider)?.provider ?? AUTO_PROVIDER
    const selectedProvider = providerDraft ?? savedProvider
    const limitText = limitDraft ?? setting.monthly_token_limit?.toString() ?? ''
    const isLimitBlank = limitText.trim() === ''
    const parsedLimit = isLimitBlank ? null : parseLimitInput(limitText, { min: 1, max: AI_FORM_LIMIT.MONTHLY_TOKEN_LIMIT_MAX })
    const hasLimitError = !isLimitBlank && parsedLimit === null
    const nextProvider = credentials.find((credential) => credential.provider === selectedProvider)?.provider ?? null
    const isDirty = parsedLimit !== setting.monthly_token_limit || selectedProvider !== savedProvider

    const handleSubmit = () =>
        save.mutate(
            { monthly_token_limit: parsedLimit, default_provider: nextProvider },
            {
                onSuccess: () => {
                    setProviderDraft(null)
                    setLimitDraft(null)
                    toast.success('AI 설정을 저장했습니다.')
                },
            },
        )

    return (
        <form
            className='flex max-w-xl flex-col gap-3'
            noValidate
            onSubmit={(event) => {
                event.preventDefault()
                handleSubmit()
            }}>
            <Field>
                <FieldLabel htmlFor='ai-default-provider'>기본 공급자</FieldLabel>
                <Select value={selectedProvider} onValueChange={setProviderDraft}>
                    <SelectTrigger id='ai-default-provider' size='sm' className='w-64 max-w-full'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={AUTO_PROVIDER}>자동 (가장 먼저 등록한 키)</SelectItem>
                        {credentials.map((credential) => (
                            <SelectItem key={credential.provider} value={credential.provider}>
                                {AI_PROVIDER_LABEL[credential.provider]}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                <FieldDescription>AI 기능은 이 공급자의 키와 모델로 호출합니다. 등록한 키 중에서만 고를 수 있습니다.</FieldDescription>
            </Field>
            <AiLimitNumberField
                id='ai-monthly-token-limit'
                label='월 토큰 상한'
                value={limitText}
                onValueChange={setLimitDraft}
                placeholder='비워 두면 운영자 기본값'
                description='입력 토큰과 출력 토큰의 합입니다. 비워 두면 운영자 기본값을 따르고, 값을 넣으면 그 값이 먼저 적용됩니다.'
                error={hasLimitError ? `1 이상 ${formatCount(AI_FORM_LIMIT.MONTHLY_TOKEN_LIMIT_MAX)} 이하의 정수를 입력하세요.` : undefined}
            />
            <div>
                <Button type='submit' size='sm' disabled={!isDirty || hasLimitError || save.isPending}>
                    저장
                </Button>
            </div>
        </form>
    )
}
