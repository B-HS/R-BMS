'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiTokenUsageLine } from '@features/ai/ai-token-usage-line'
import { AiUsageMeter } from '@features/ai/ai-usage-meter'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { AiUserSettingForm } from '@widgets/settings/ai-user-setting-form'
import { useAiCredentialList, useAiUsageSummary, useAiUserSetting } from '@entities/ai.query'

export const AiUsagePanel: FC = () => {
    const t = useTranslations('settings.usagePanel')
    const usage = useAiUsageSummary()
    const setting = useAiUserSetting()
    const credentials = useAiCredentialList()

    const isLoading = usage.isLoading || setting.isLoading
    const isError = usage.isError || setting.isError
    const summary = usage.data
    const savedSetting = setting.data

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-3'>
            {isLoading && <TableSkeleton rows={3} label={t('loading')} />}
            {!isLoading && isError && (
                <LoadErrorState
                    title={t('loadError')}
                    isRetrying={usage.isFetching || setting.isFetching}
                    onRetry={() => void Promise.all([usage.refetch(), setting.refetch()])}
                />
            )}
            {summary && (
                <div className='flex max-w-xl flex-col gap-3'>
                    <div className='flex flex-col gap-1'>
                        <AiUsageMeter label={t('todayCalls')} used={summary.today.calls} limit={summary.limits.daily_calls} unit={t('callsUnit')} />
                        <AiTokenUsageLine
                            inputTokens={summary.today.input_tokens}
                            outputTokens={summary.today.output_tokens}
                            cachedInputTokens={summary.today.cached_input_tokens}
                        />
                    </div>
                    <div className='flex flex-col gap-1'>
                        <AiUsageMeter
                            label={t('monthTokens')}
                            used={summary.this_month.input_tokens + summary.this_month.output_tokens}
                            limit={summary.limits.monthly_token_limit}
                            unit={t('tokensUnit')}
                        />
                        <AiTokenUsageLine
                            inputTokens={summary.this_month.input_tokens}
                            outputTokens={summary.this_month.output_tokens}
                            cachedInputTokens={summary.this_month.cached_input_tokens}
                        />
                    </div>
                    <p className='text-muted-foreground text-xs'>{t('concurrentRuns', { count: summary.limits.concurrent_runs })}</p>
                </div>
            )}
            {savedSetting && <AiUserSettingForm setting={savedSetting} credentials={credentials.data ?? []} />}
            <PanelFootnote>{t('footnote')}</PanelFootnote>
        </PanelCard>
    )
}
