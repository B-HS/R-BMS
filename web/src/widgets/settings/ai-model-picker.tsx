'use client'

import { useId, useState, type FC } from 'react'
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

    const manualInputId = useId()
    const { data, isLoading, isError, error, isFetching, refetch } = useAiModelList(provider, isOpen)
    const setModel = useSetAiCredentialModel()

    const providerLabel = AI_PROVIDER_LABEL[provider]
    const openLabel = model ? '모델 변경' : '모델 선택'
    const toggleLabel = isOpen ? '모델 목록 닫기' : openLabel

    const handleModelChange = (nextModel: string) =>
        setModel.mutate(
            { provider, model: nextModel },
            {
                onSuccess: () => {
                    setIsOpen(false)
                    toast.success(`${providerLabel} 모델을 ${nextModel} 로 바꿨습니다.`)
                },
            },
        )

    return (
        <div className='flex flex-col gap-2'>
            <div className='flex flex-wrap items-center gap-2 text-xs'>
                <span className='text-muted-foreground'>모델</span>
                {model ? <code className='font-mono'>{model}</code> : <Badge variant='outline'>선택 안 됨</Badge>}
                <Button
                    variant='outline'
                    size='xs'
                    aria-expanded={isOpen}
                    aria-label={`${providerLabel} ${toggleLabel}`}
                    onClick={() => setIsOpen(!isOpen)}>
                    {isOpen ? '닫기' : openLabel}
                </Button>
            </div>
            {model === null && !isOpen && <p className='text-warning text-xs break-keep'>모델을 고르지 않으면 이 키로 AI 를 호출할 수 없습니다.</p>}
            {isOpen && isLoading && <TableSkeleton rows={1} label={`${providerLabel} 모델 목록 불러오는 중`} />}
            {isOpen && isError && (
                <LoadErrorState
                    title='모델 목록을 불러오지 못했습니다'
                    className='min-h-0 p-3'
                    description={error instanceof ApiError ? error.message : undefined}
                    isRetrying={isFetching}
                    onRetry={() => void refetch()}
                />
            )}
            {isOpen && data && data.length === 0 && <p className='text-muted-foreground text-xs'>이 키로 쓸 수 있는 모델이 목록에 없습니다.</p>}
            {isOpen && (isError || data?.length === 0) && (
                <AiModelIdForm
                    inputId={manualInputId}
                    label={`${providerLabel} 모델 id 직접 입력`}
                    isPending={setModel.isPending}
                    onSubmitModel={handleModelChange}
                />
            )}
            {isOpen && data && data.length > 0 && (
                <>
                    <AiModelSelect models={data} value={model} disabled={setModel.isPending} onValueChange={handleModelChange} />
                    <p className='text-muted-foreground text-xs'>
                        &apos;형식 지원 확인 안 됨&apos;은 공급자가 모델별 구조화 출력 지원 여부를 알려 주지 않는다는 뜻입니다. 실패하면 다른 모델을
                        고르세요.
                    </p>
                </>
            )}
        </div>
    )
}
