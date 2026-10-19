import type { FC } from 'react'
import { AI_DATA_NEED_LABEL } from '@entities/ai.type'

const FALLBACK = '아직 해석할 기록이 모자랍니다.'

export const AiDataNotice: FC<{ need: string }> = ({ need }) => (
    <p role='status' className='text-muted-foreground border-border border border-dashed p-2 text-xs break-keep'>
        {AI_DATA_NEED_LABEL[need] ?? FALLBACK} 이 상태에서는 AI 를 부르지 않습니다.
    </p>
)
