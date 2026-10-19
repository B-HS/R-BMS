'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { AiUsageMeter } from '@features/ai/ai-usage-meter'
import { PanelCard } from '@features/shell/panel-card'
import { Button } from '@shared/ui/button'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'
import { useAiCredentialList, useAiUsageSummary, useAiUserSetting } from '@entities/ai.query'
import { AI_PROVIDER_LABEL, selectActiveAiCredential } from '@entities/ai.type'

export const AiActiveProviderPanel: FC = () => {
    const credentials = useAiCredentialList()
    const setting = useAiUserSetting()
    const usage = useAiUsageSummary()

    const active = selectActiveAiCredential(credentials.data ?? [], setting.data?.default_provider ?? null)

    return (
        <PanelCard
            title='사용 중인 AI'
            action={
                <Button asChild variant='ghost' size='sm'>
                    <Link href={AI_SETTINGS_HREF}>AI 설정</Link>
                </Button>
            }
            contentClassName='flex max-w-xl flex-col gap-3'>
            {active && (
                <p className='text-xs'>
                    {AI_PROVIDER_LABEL[active.provider]} · <code className='font-mono'>{active.model ?? '모델 미선택'}</code>
                </p>
            )}
            {usage.data && <AiUsageMeter label='오늘 호출' used={usage.data.today.calls} limit={usage.data.limits.daily_calls} unit='회' />}
        </PanelCard>
    )
}
