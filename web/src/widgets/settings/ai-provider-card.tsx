'use client'

import { useId, useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { AiCredentialStatus } from '@features/ai/ai-credential-status'
import { AiKeyForm } from '@features/ai/ai-key-form'
import { ConfirmAction } from '@features/dialog/confirm-action'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { AiModelPicker } from '@widgets/settings/ai-model-picker'
import { useRegisterAiCredential, useRemoveAiCredential, useRevalidateAiCredential } from '@entities/ai.query'
import { AI_PROVIDER_LABEL, AI_PROVIDER_NOTE, type AiCredential, type AiProviderId } from '@entities/ai.type'

type AiProviderCardProps = {
    provider: AiProviderId
    credential: AiCredential | undefined
    isActive: boolean
}

export const AiProviderCard: FC<AiProviderCardProps> = ({ provider, credential, isActive }) => {
    const [isReplacing, setIsReplacing] = useState(false)
    const [isRemoveOpen, setIsRemoveOpen] = useState(false)

    const t = useTranslations('settings.providerCard')
    const headingId = useId()
    const register = useRegisterAiCredential()
    const revalidate = useRevalidateAiCredential()
    const remove = useRemoveAiCredential()

    const label = AI_PROVIDER_LABEL[provider]

    const handleSubmitKey = (apiKey: string) =>
        register.mutate(
            { provider, apiKey },
            {
                onSuccess: (saved) => {
                    setIsReplacing(false)
                    if (saved.last_error_kind === 'forbidden') {
                        toast.warning(t('savedForbidden', { provider: label }))
                        return
                    }
                    toast.success(t(saved.model ? 'saved' : 'savedNeedsModel', { provider: label }))
                },
                onSettled: () => register.reset(),
            },
        )

    const handleRemove = () =>
        remove.mutate(provider, {
            onSuccess: () => {
                setIsRemoveOpen(false)
                setIsReplacing(false)
                toast.success(t('removed', { provider: label }))
            },
        })

    return (
        <section aria-labelledby={headingId} className='border-border flex flex-col gap-3 border p-3'>
            <div className='flex flex-wrap items-center gap-2'>
                <h3 id={headingId} className='text-sm font-semibold'>
                    {label}
                </h3>
                <Badge variant={credential ? 'secondary' : 'outline'}>{credential ? t('registered') : t('unregistered')}</Badge>
                {isActive && <Badge>{t('active')}</Badge>}
            </div>
            <p className='text-muted-foreground text-xs break-keep'>{AI_PROVIDER_NOTE[provider]}</p>
            {credential && (
                <>
                    <AiCredentialStatus credential={credential} />
                    <AiModelPicker provider={provider} model={credential.model} />
                    {!isReplacing && (
                        <div className='flex flex-wrap gap-1'>
                            <Button
                                variant='outline'
                                size='sm'
                                aria-label={t('replaceLabel', { provider: label })}
                                onClick={() => setIsReplacing(true)}>
                                {t('replace')}
                            </Button>
                            <Button
                                variant='outline'
                                size='sm'
                                disabled={revalidate.isPending}
                                aria-label={t('revalidateLabel', { provider: label })}
                                onClick={() =>
                                    revalidate.mutate(provider, {
                                        onSuccess: (checked) =>
                                            checked.last_error_kind === 'forbidden'
                                                ? toast.warning(t('validForbidden', { provider: label }))
                                                : toast.success(t('valid', { provider: label })),
                                    })
                                }>
                                {t('revalidate')}
                            </Button>
                            <Button
                                variant='ghost'
                                size='sm'
                                aria-label={t('removeLabel', { provider: label })}
                                onClick={() => setIsRemoveOpen(true)}>
                                {t('remove')}
                            </Button>
                        </div>
                    )}
                </>
            )}
            {(!credential || isReplacing) && (
                <AiKeyForm
                    inputId={`${headingId}-api-key`}
                    label={t(credential ? 'newKeyLabel' : 'keyLabel', { provider: label })}
                    submitLabel={credential ? t('submitReplace') : t('submitRegister')}
                    isPending={register.isPending}
                    onSubmitKey={handleSubmitKey}
                    onCancel={credential ? () => setIsReplacing(false) : undefined}
                />
            )}
            <ConfirmAction
                open={isRemoveOpen}
                onOpenChange={setIsRemoveOpen}
                title={t('removeTitle', { provider: label })}
                description={t('removeDescription')}
                confirmLabel={t('removeConfirm')}
                confirmDisabled={remove.isPending}
                onConfirm={handleRemove}
            />
        </section>
    )
}
