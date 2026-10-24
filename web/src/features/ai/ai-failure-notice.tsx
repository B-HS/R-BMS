import type { FC } from 'react'
import Link from 'next/link'
import { TriangleAlert } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { Alert, AlertDescription, AlertTitle } from '@shared/ui/alert'
import { Button } from '@shared/ui/button'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'
import { AI_ERROR_CODE, toAiFailureView } from '@entities/ai.type'

export const AiFailureNotice: FC<{ error: unknown }> = ({ error }) => {
    const t = useTranslations('pages.ai.failureNotice')
    const failure = toAiFailureView(error)

    return (
        <Alert role='alert' variant='destructive'>
            <TriangleAlert aria-hidden />
            <AlertTitle>{t('title')}</AlertTitle>
            <AlertDescription className='flex flex-col gap-2 break-keep'>
                <p className='text-foreground'>{failure.message ?? t('fallback')}</p>
                {failure.retryAfterSeconds !== null && <p>{t('retryAfter', { seconds: failure.retryAfterSeconds })}</p>}
                {failure.code === AI_ERROR_CODE.MODEL_UNAVAILABLE && <p>{t('reselectModel')}</p>}
                {failure.needsSettings && (
                    <div>
                        <Button asChild variant='outline' size='sm'>
                            <Link href={AI_SETTINGS_HREF}>{t('openSettings')}</Link>
                        </Button>
                    </div>
                )}
            </AlertDescription>
        </Alert>
    )
}
