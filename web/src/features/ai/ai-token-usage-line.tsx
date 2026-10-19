import type { FC } from 'react'
import { formatCount } from '@shared/lib/format'

type AiTokenUsageLineProps = {
    inputTokens: number
    outputTokens: number
    cachedInputTokens: number
}

export const AiTokenUsageLine: FC<AiTokenUsageLineProps> = ({ inputTokens, outputTokens, cachedInputTokens }) => (
    <p className='text-muted-foreground text-xs tabular-nums'>
        입력 {formatCount(inputTokens)} · 출력 {formatCount(outputTokens)} 토큰(캐시 {formatCount(cachedInputTokens)})
    </p>
)
