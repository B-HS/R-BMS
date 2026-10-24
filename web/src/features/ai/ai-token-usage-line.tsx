import type { FC } from 'react'
import { useTranslations } from 'next-intl'

type AiTokenUsageLineProps = {
    inputTokens: number
    outputTokens: number
    cachedInputTokens: number
}

export const AiTokenUsageLine: FC<AiTokenUsageLineProps> = ({ inputTokens, outputTokens, cachedInputTokens }) => {
    const t = useTranslations('pages.ai')

    return (
        <p className='text-muted-foreground text-xs tabular-nums'>
            {t('tokenUsage', { input: inputTokens, output: outputTokens, cached: cachedInputTokens })}
        </p>
    )
}
