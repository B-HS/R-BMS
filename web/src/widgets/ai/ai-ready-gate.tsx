'use client'

import type { FC, PropsWithChildren } from 'react'
import { useTranslations } from 'next-intl'
import { AiSetupNotice } from '@features/ai/ai-setup-notice'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { useAiCredentialList, useAiUserSetting } from '@entities/ai.query'
import { selectActiveAiCredential } from '@entities/ai.type'

export const AiReadyGate: FC<PropsWithChildren> = ({ children }) => {
    const t = useTranslations('ai.readyGate')
    const credentials = useAiCredentialList()
    const setting = useAiUserSetting()

    const isLoading = credentials.isLoading || setting.isLoading
    const isError = credentials.isError || setting.isError
    const active = selectActiveAiCredential(credentials.data ?? [], setting.data?.default_provider ?? null)

    return (
        <>
            {isLoading && <PanelSkeleton label={t('loading')} />}
            {!isLoading && isError && (
                <LoadErrorState
                    title={t('loadError')}
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
