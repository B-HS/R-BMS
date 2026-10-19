'use client'

import type { FC, PropsWithChildren } from 'react'
import { AiSetupNotice } from '@features/ai/ai-setup-notice'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { useAiCredentialList, useAiUserSetting } from '@entities/ai.query'
import { selectActiveAiCredential } from '@entities/ai.type'

export const AiReadyGate: FC<PropsWithChildren> = ({ children }) => {
    const credentials = useAiCredentialList()
    const setting = useAiUserSetting()

    const isLoading = credentials.isLoading || setting.isLoading
    const isError = credentials.isError || setting.isError
    const active = selectActiveAiCredential(credentials.data ?? [], setting.data?.default_provider ?? null)

    return (
        <>
            {isLoading && <PanelSkeleton label='AI 설정 불러오는 중' />}
            {!isLoading && isError && (
                <LoadErrorState
                    title='AI 설정을 불러오지 못했습니다'
                    isRetrying={credentials.isFetching || setting.isFetching}
                    onRetry={() => void Promise.all([credentials.refetch(), setting.refetch()])}
                />
            )}
            {!isLoading && !isError && active === null && <AiSetupNotice reason='no_key' />}
            {!isLoading && !isError && active !== null && active.model === null && <AiSetupNotice reason='no_model' />}
            {!isLoading && !isError && active?.model && children}
        </>
    )
}
