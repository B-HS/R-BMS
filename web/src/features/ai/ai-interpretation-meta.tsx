import type { FC } from 'react'
import { AiTokenUsageLine } from '@features/ai/ai-token-usage-line'
import { Badge } from '@shared/ui/badge'
import { formatDateTime } from '@shared/lib/format'
import { AI_PROVIDER_LABEL, type AiInterpretationMeta as AiInterpretationMetaData } from '@entities/ai.type'

export const AiInterpretationMeta: FC<{ meta: AiInterpretationMetaData }> = ({ meta }) => (
    <div className='flex flex-wrap items-center gap-2'>
        <span className='text-muted-foreground text-xs'>
            {AI_PROVIDER_LABEL[meta.provider]} · <code className='font-mono'>{meta.model}</code> · {formatDateTime(meta.created_at)}
        </span>
        {meta.cached && <Badge variant='outline'>저장된 해석</Badge>}
        {!meta.is_current && <Badge variant='secondary'>기록이나 설정이 바뀜 · 다시 만들 수 있음</Badge>}
        <AiTokenUsageLine {...meta.usage} />
    </div>
)
