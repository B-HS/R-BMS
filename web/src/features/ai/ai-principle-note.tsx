import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { PanelCard } from '@features/shell/panel-card'

export const AiPrincipleNote: FC = () => {
    const t = useTranslations('pages.ai.principle')

    return (
        <PanelCard title={t('title')}>
            <ul className='text-muted-foreground flex list-disc flex-col gap-1 pl-4 text-xs leading-5 break-keep'>
                <li>{t('computed')}</li>
                <li>{t('interpretation')}</li>
                <li>{t('onRequest')}</li>
            </ul>
        </PanelCard>
    )
}
