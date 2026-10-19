'use client'

import type { FC } from 'react'
import { AiProviderNotice } from '@features/ai/ai-provider-notice'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { AiProviderCard } from '@widgets/settings/ai-provider-card'
import { useAiCredentialList, useAiUserSetting } from '@entities/ai.query'
import { AI_PROVIDER_ORDER, selectActiveAiCredential } from '@entities/ai.type'

export const AiCredentialPanel: FC = () => {
    const { data, isLoading, isError, isFetching, refetch } = useAiCredentialList()
    const { data: setting } = useAiUserSetting()

    const credentials = data ?? []
    const activeProvider = selectActiveAiCredential(credentials, setting?.default_provider ?? null)?.provider

    return (
        <PanelCard title='AI 키' contentClassName='flex flex-col gap-3'>
            <AiProviderNotice />
            {isLoading && <TableSkeleton rows={3} label='AI 키 불러오는 중' />}
            {!isLoading && isError && (
                <LoadErrorState title='AI 키 목록을 불러오지 못했습니다' isRetrying={isFetching} onRetry={() => void refetch()} />
            )}
            {!isLoading &&
                !isError &&
                AI_PROVIDER_ORDER.map((provider) => (
                    <AiProviderCard
                        key={provider}
                        provider={provider}
                        credential={credentials.find((credential) => credential.provider === provider)}
                        isActive={provider === activeProvider}
                    />
                ))}
            <PanelFootnote>키 등록, 다시 검증, 모델 목록 조회는 분당 횟수 제한이 있어 잠시 뒤 다시 시도해야 할 수 있습니다.</PanelFootnote>
        </PanelCard>
    )
}
