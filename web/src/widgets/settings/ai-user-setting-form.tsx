'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
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

    const t = useTranslations('settings.userSettingForm')
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
                    toast.success(t('saved'))
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
                <FieldLabel htmlFor='ai-default-provider'>{t('providerLabel')}</FieldLabel>
                <Select value={selectedProvider} onValueChange={setProviderDraft}>
                    <SelectTrigger id='ai-default-provider' size='sm' className='w-64 max-w-full'>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={AUTO_PROVIDER}>{t('providerAuto')}</SelectItem>
                        {credentials.map((credential) => (
                            <SelectItem key={credential.provider} value={credential.provider}>
                                {AI_PROVIDER_LABEL[credential.provider]}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                <FieldDescription>{t('providerDescription')}</FieldDescription>
            </Field>
            <AiLimitNumberField
                id='ai-monthly-token-limit'
                label={t('limitLabel')}
                value={limitText}
                onValueChange={setLimitDraft}
                placeholder={t('limitPlaceholder')}
                description={t('limitDescription')}
                error={hasLimitError ? t('limitError', { max: formatCount(AI_FORM_LIMIT.MONTHLY_TOKEN_LIMIT_MAX) }) : undefined}
            />
            <div>
                <Button type='submit' size='sm' disabled={!isDirty || hasLimitError || save.isPending}>
                    {t('save')}
                </Button>
            </div>
        </form>
    )
}
