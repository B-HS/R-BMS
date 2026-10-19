'use client'

import type { FC } from 'react'
import { AiTokenUsageLine } from '@features/ai/ai-token-usage-line'
import { AiUsageMeter } from '@features/ai/ai-usage-meter'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { AiUserSettingForm } from '@widgets/settings/ai-user-setting-form'
import { useAiCredentialList, useAiUsageSummary, useAiUserSetting } from '@entities/ai.query'

export const AiUsagePanel: FC = () => {
    const usage = useAiUsageSummary()
    const setting = useAiUserSetting()
    const credentials = useAiCredentialList()

    const isLoading = usage.isLoading || setting.isLoading
    const isError = usage.isError || setting.isError
    const summary = usage.data
    const savedSetting = setting.data

    return (
        <PanelCard title='AI 사용량과 설정' contentClassName='flex flex-col gap-3'>
            {isLoading && <TableSkeleton rows={3} label='AI 사용량 불러오는 중' />}
            {!isLoading && isError && (
                <LoadErrorState
                    title='AI 사용량을 불러오지 못했습니다'
                    isRetrying={usage.isFetching || setting.isFetching}
                    onRetry={() => void Promise.all([usage.refetch(), setting.refetch()])}
                />
            )}
            {summary && (
                <div className='flex max-w-xl flex-col gap-3'>
                    <div className='flex flex-col gap-1'>
                        <AiUsageMeter label='오늘 호출' used={summary.today.calls} limit={summary.limits.daily_calls} unit='회' />
                        <AiTokenUsageLine
                            inputTokens={summary.today.input_tokens}
                            outputTokens={summary.today.output_tokens}
                            cachedInputTokens={summary.today.cached_input_tokens}
                        />
                    </div>
                    <div className='flex flex-col gap-1'>
                        <AiUsageMeter
                            label='이번 달 토큰'
                            used={summary.this_month.input_tokens + summary.this_month.output_tokens}
                            limit={summary.limits.monthly_token_limit}
                            unit='토큰'
                        />
                        <AiTokenUsageLine
                            inputTokens={summary.this_month.input_tokens}
                            outputTokens={summary.this_month.output_tokens}
                            cachedInputTokens={summary.this_month.cached_input_tokens}
                        />
                    </div>
                    <p className='text-muted-foreground text-xs'>동시 실행 한도 {summary.limits.concurrent_runs}건</p>
                </div>
            )}
            {savedSetting && <AiUserSettingForm setting={savedSetting} credentials={credentials.data ?? []} />}
            <PanelFootnote>
                집계는 UTC 기준입니다. 하루 호출 수는 UTC 자정(한국 시간 오전 9시)에, 월 토큰은 UTC 월초에 처음부터 다시 셉니다. 호출 수는 공급자에게
                요청을 보낼 때마다 세며 실패한 호출도 포함하고, 저장된 결과를 다시 보여 줄 때는 세지 않습니다. 월 토큰 상한은 호출 전에 합계로
                검사하므로 마지막 한 번이 상한을 조금 넘길 수 있습니다.
            </PanelFootnote>
        </PanelCard>
    )
}
