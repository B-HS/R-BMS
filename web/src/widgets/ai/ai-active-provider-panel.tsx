'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { AiUsageMeter } from '@features/ai/ai-usage-meter'
import { PanelCard } from '@features/shell/panel-card'
import { Button } from '@shared/ui/button'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'
import { useAiCredentialList, useAiUsageSummary, useAiUserSetting } from '@entities/ai.query'
import { AI_PROVIDER_LABEL, selectActiveAiCredential } from '@entities/ai.type'

export const AiActiveProviderPanel: FC = () => {
    const t = useTranslations('ai.activeProvider')
    const credentials = useAiCredentialList()
    const setting = useAiUserSetting()
    const usage = useAiUsageSummary()

    const active = selectActiveAiCredential(credentials.data ?? [], setting.data?.default_provider ?? null)

    return (
        <PanelCard
            title={t('title')}
            action={
                <Button asChild variant='ghost' size='sm'>
                    <Link href={AI_SETTINGS_HREF}>{t('settingsLink')}</Link>
                </Button>
            }
            contentClassName='flex max-w-xl flex-col gap-3'>
            {active && (
                <p className='text-xs'>
                    {AI_PROVIDER_LABEL[active.provider]} · <code className='font-mono'>{active.model ?? t('modelNotSelected')}</code>
                </p>
            )}
            {usage.data && (
                <AiUsageMeter label={t('todayCalls')} used={usage.data.today.calls} limit={usage.data.limits.daily_calls} unit={t('callsUnit')} />
            )}
        </PanelCard>
    )
}
