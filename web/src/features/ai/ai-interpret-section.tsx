import type { FC, PropsWithChildren } from 'react'
import { useTranslations } from 'next-intl'
import { AiFailureNotice } from '@features/ai/ai-failure-notice'
import { AiInterpretationMeta } from '@features/ai/ai-interpretation-meta'
import { Button } from '@shared/ui/button'
import type { AiInterpretationMeta as AiInterpretationMetaData } from '@entities/ai.type'

type AiInterpretSectionProps = PropsWithChildren<{
    label: string
    meta: AiInterpretationMetaData | null
    isPending: boolean
    error: unknown
    isBlocked: boolean
    onInterpret: () => void
}>

/** The AI half of a feature panel: the button the user presses, the failure, and the answer with how it was made. Nothing runs on its own. */
export const AiInterpretSection: FC<AiInterpretSectionProps> = ({ label, meta, isPending, error, isBlocked, onInterpret, children }) => {
    const t = useTranslations('pages.ai.interpret')

    return (
        <section aria-label={label} className='flex flex-col gap-2'>
            <div className='flex flex-wrap items-center gap-2'>
                <Button type='button' size='sm' disabled={isPending || isBlocked} onClick={onInterpret}>
                    {isPending && t('generating')}
                    {!isPending && (meta ? t('regenerate') : t('generate'))}
                </Button>
                <span className='text-muted-foreground text-xs'>{t('hint')}</span>
            </div>
            {error !== null && error !== undefined && <AiFailureNotice error={error} />}
            {meta && <AiInterpretationMeta meta={meta} />}
            {children}
        </section>
    )
}
