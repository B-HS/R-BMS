'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiProviderNotice } from '@features/ai/ai-provider-notice'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { AiProviderCard } from '@widgets/settings/ai-provider-card'
import { useAiCredentialList, useAiUserSetting } from '@entities/ai.query'
import { AI_PROVIDER_ORDER, selectActiveAiCredential } from '@entities/ai.type'

export const AiCredentialPanel: FC = () => {
    const t = useTranslations('settings.credentialPanel')
    const { data, isLoading, isError, isFetching, refetch } = useAiCredentialList()
    const { data: setting } = useAiUserSetting()

    const credentials = data ?? []
    const activeProvider = selectActiveAiCredential(credentials, setting?.default_provider ?? null)?.provider

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-3'>
            <AiProviderNotice />
            {isLoading && <TableSkeleton rows={3} label={t('loading')} />}
            {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
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
            <PanelFootnote>{t('footnote')}</PanelFootnote>
        </PanelCard>
    )
}
