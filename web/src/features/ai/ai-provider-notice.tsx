import type { FC } from 'react'
import { ShieldCheck } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { Alert, AlertDescription, AlertTitle } from '@shared/ui/alert'

export const AiProviderNotice: FC = () => {
    const t = useTranslations('pages.ai.providerNotice')

    return (
        <Alert role='note'>
            <ShieldCheck aria-hidden />
            <AlertTitle>{t('title')}</AlertTitle>
            <AlertDescription className='break-keep'>
                <p>{t('storage')}</p>
                <p>{t('billing')}</p>
            </AlertDescription>
        </Alert>
    )
}
