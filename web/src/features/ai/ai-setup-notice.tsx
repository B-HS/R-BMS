import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { EmptyState } from '@features/state/empty-state'
import { Button } from '@shared/ui/button'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'

export const AiSetupNotice: FC<{ reason: 'no_key' | 'no_model' }> = ({ reason }) => {
    const t = useTranslations('pages.ai.setup')

    return (
        <EmptyState
            title={t(`${reason}.title`)}
            description={t(`${reason}.description`)}
            action={
                <Button asChild size='sm'>
                    <Link href={AI_SETTINGS_HREF}>{t(`${reason}.action`)}</Link>
                </Button>
            }
        />
    )
}
