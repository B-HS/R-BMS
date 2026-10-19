import type { FC, PropsWithChildren } from 'react'
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
export const AiInterpretSection: FC<AiInterpretSectionProps> = ({ label, meta, isPending, error, isBlocked, onInterpret, children }) => (
    <section aria-label={label} className='flex flex-col gap-2'>
        <div className='flex flex-wrap items-center gap-2'>
            <Button type='button' size='sm' disabled={isPending || isBlocked} onClick={onInterpret}>
                {isPending && 'AI 가 만드는 중'}
                {!isPending && (meta ? 'AI 해석 다시 만들기' : 'AI 해석 만들기')}
            </Button>
            <span className='text-muted-foreground text-xs'>누를 때만 내 키로 한 번 호출합니다. 같은 입력이면 저장된 해석을 씁니다.</span>
        </div>
        {error !== null && error !== undefined && <AiFailureNotice error={error} />}
        {meta && <AiInterpretationMeta meta={meta} />}
        {children}
    </section>
)
