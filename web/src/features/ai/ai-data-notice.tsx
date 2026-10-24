import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { isAiDataNeed } from '@entities/ai.type'

export const AiDataNotice: FC<{ need: string }> = ({ need }) => {
    const t = useTranslations('pages.ai')

    return (
        <p role='status' className='text-muted-foreground border-border border border-dashed p-2 text-xs break-keep'>
            {t('dataNotice', { reason: isAiDataNeed(need) ? t(`dataNeed.${need}`) : t('dataNeed.fallback') })}
        </p>
    )
}
