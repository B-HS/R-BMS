'use client'

import { useId, useState, type FC } from 'react'
import { toast } from 'sonner'
import { AiCredentialStatus } from '@features/ai/ai-credential-status'
import { AiKeyForm } from '@features/ai/ai-key-form'
import { ConfirmAction } from '@features/dialog/confirm-action'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { AiModelPicker } from '@widgets/settings/ai-model-picker'
import { useRegisterAiCredential, useRemoveAiCredential, useRevalidateAiCredential } from '@entities/ai.query'
import { AI_PROVIDER_LABEL, AI_PROVIDER_NOTE, type AiCredential, type AiProviderId } from '@entities/ai.type'

const FORBIDDEN_HINT = '키 권한 또는 모델 접근을 확인하세요.'

type AiProviderCardProps = {
    provider: AiProviderId
    credential: AiCredential | undefined
    isActive: boolean
}

export const AiProviderCard: FC<AiProviderCardProps> = ({ provider, credential, isActive }) => {
    const [isReplacing, setIsReplacing] = useState(false)
    const [isRemoveOpen, setIsRemoveOpen] = useState(false)

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
                        toast.warning(`${label} 키를 저장했지만 공급자가 일부 요청을 막았습니다. ${FORBIDDEN_HINT}`)
                        return
                    }
                    toast.success(saved.model ? `${label} 키를 저장했습니다.` : `${label} 키를 저장했습니다. 모델을 고르면 AI 기능에 쓸 수 있습니다.`)
                },
                onSettled: () => register.reset(),
            },
        )

    const handleRemove = () =>
        remove.mutate(provider, {
            onSuccess: () => {
                setIsRemoveOpen(false)
                setIsReplacing(false)
                toast.success(`${label} 키를 삭제했습니다.`)
            },
        })

    return (
        <section aria-labelledby={headingId} className='border-border flex flex-col gap-3 border p-3'>
            <div className='flex flex-wrap items-center gap-2'>
                <h3 id={headingId} className='text-sm font-semibold'>
                    {label}
                </h3>
                <Badge variant={credential ? 'secondary' : 'outline'}>{credential ? '등록됨' : '미등록'}</Badge>
                {isActive && <Badge>사용 중</Badge>}
            </div>
            <p className='text-muted-foreground text-xs break-keep'>{AI_PROVIDER_NOTE[provider]}</p>
            {credential && (
                <>
                    <AiCredentialStatus credential={credential} />
                    <AiModelPicker provider={provider} model={credential.model} />
                    {!isReplacing && (
                        <div className='flex flex-wrap gap-1'>
                            <Button variant='outline' size='sm' aria-label={`${label} 키 교체`} onClick={() => setIsReplacing(true)}>
                                키 교체
                            </Button>
                            <Button
                                variant='outline'
                                size='sm'
                                disabled={revalidate.isPending}
                                aria-label={`${label} 키 다시 검증`}
                                onClick={() =>
                                    revalidate.mutate(provider, {
                                        onSuccess: (checked) =>
                                            checked.last_error_kind === 'forbidden'
                                                ? toast.warning(`${label} 키는 유효하지만 공급자가 일부 요청을 막았습니다. ${FORBIDDEN_HINT}`)
                                                : toast.success(`${label} 키가 유효합니다.`),
                                    })
                                }>
                                다시 검증
                            </Button>
                            <Button variant='ghost' size='sm' aria-label={`${label} 키 삭제`} onClick={() => setIsRemoveOpen(true)}>
                                삭제
                            </Button>
                        </div>
                    )}
                </>
            )}
            {(!credential || isReplacing) && (
                <AiKeyForm
                    inputId={`${headingId}-api-key`}
                    label={credential ? `새 ${label} API 키` : `${label} API 키`}
                    submitLabel={credential ? '교체' : '등록'}
                    isPending={register.isPending}
                    onSubmitKey={handleSubmitKey}
                    onCancel={credential ? () => setIsReplacing(false) : undefined}
                />
            )}
            <ConfirmAction
                open={isRemoveOpen}
                onOpenChange={setIsRemoveOpen}
                title={`${label} 키를 삭제할까요?`}
                description='서버에 저장된 암호화된 키를 지우며 되살릴 수 없습니다. 공급자 콘솔의 키는 그대로이므로 필요하면 콘솔에서 직접 취소하세요.'
                confirmLabel='삭제'
                confirmDisabled={remove.isPending}
                onConfirm={handleRemove}
            />
        </section>
    )
}
