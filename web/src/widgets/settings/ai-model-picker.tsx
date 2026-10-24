'use client'

import { useId, useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { AiModelIdForm } from '@features/ai/ai-model-id-form'
import { AiModelSelect } from '@features/ai/ai-model-select'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { ApiError } from '@shared/lib/fetch'
import { useAiModelList, useSetAiCredentialModel } from '@entities/ai.query'
import { AI_PROVIDER_LABEL, type AiProviderId } from '@entities/ai.type'

type AiModelPickerProps = {
    provider: AiProviderId
    model: string | null
}

export const AiModelPicker: FC<AiModelPickerProps> = ({ provider, model }) => {
    const [isOpen, setIsOpen] = useState(false)

    const t = useTranslations('settings.modelPicker')
    const manualInputId = useId()
    const { data, isLoading, isError, error, isFetching, refetch } = useAiModelList(provider, isOpen)
    const setModel = useSetAiCredentialModel()

    const providerLabel = AI_PROVIDER_LABEL[provider]
    const openAction = model ? 'change' : 'select'

    const handleModelChange = (nextModel: string) =>
        setModel.mutate(
            { provider, model: nextModel },
            {
                onSuccess: () => {
                    setIsOpen(false)
                    toast.success(t('changed', { provider: providerLabel, model: nextModel }))
                },
            },
        )

    return (
        <div className='flex flex-col gap-2'>
            <div className='flex flex-wrap items-center gap-2 text-xs'>
                <span className='text-muted-foreground'>{t('modelLabel')}</span>
                {model ? <code className='font-mono'>{model}</code> : <Badge variant='outline'>{t('notSelected')}</Badge>}
                <Button
                    variant='outline'
                    size='xs'
                    aria-expanded={isOpen}
                    aria-label={isOpen ? t('closeListLabel', { provider: providerLabel }) : t(`${openAction}Label`, { provider: providerLabel })}
                    onClick={() => setIsOpen(!isOpen)}>
                    {isOpen ? t('close') : t(openAction)}
                </Button>
            </div>
            {model === null && !isOpen && <p className='text-warning text-xs break-keep'>{t('noModelWarning')}</p>}
            {isOpen && isLoading && <TableSkeleton rows={1} label={t('listLoading', { provider: providerLabel })} />}
            {isOpen && isError && (
                <LoadErrorState
                    title={t('listError')}
                    className='min-h-0 p-3'
                    description={error instanceof ApiError ? error.message : undefined}
                    isRetrying={isFetching}
                    onRetry={() => void refetch()}
                />
            )}
            {isOpen && data && data.length === 0 && <p className='text-muted-foreground text-xs'>{t('listEmpty')}</p>}
            {isOpen && (isError || data?.length === 0) && (
                <AiModelIdForm
                    inputId={manualInputId}
                    label={t('manualInputLabel', { provider: providerLabel })}
                    isPending={setModel.isPending}
                    onSubmitModel={handleModelChange}
                />
            )}
            {isOpen && data && data.length > 0 && (
                <>
                    <AiModelSelect models={data} value={model} disabled={setModel.isPending} onValueChange={handleModelChange} />
                    <p className='text-muted-foreground text-xs'>{t('unverifiedNote')}</p>
                </>
            )}
        </div>
    )
}
