import type { FC } from 'react'
import Link from 'next/link'
import { EmptyState } from '@features/state/empty-state'
import { Button } from '@shared/ui/button'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'

const SETUP_COPY = {
    no_key: {
        title: '등록된 AI 키가 없습니다',
        description: 'Anthropic, OpenAI, Ollama Cloud 중 하나의 API 키를 설정에서 등록하면 AI 해석을 쓸 수 있습니다.',
        actionLabel: 'AI 키 등록하기',
    },
    no_model: {
        title: '사용할 모델을 고르지 않았습니다',
        description: '키는 등록되어 있습니다. 설정에서 모델을 고르면 AI 해석을 쓸 수 있습니다.',
        actionLabel: '모델 고르기',
    },
}

export const AiSetupNotice: FC<{ reason: keyof typeof SETUP_COPY }> = ({ reason }) => (
    <EmptyState
        title={SETUP_COPY[reason].title}
        description={SETUP_COPY[reason].description}
        action={
            <Button asChild size='sm'>
                <Link href={AI_SETTINGS_HREF}>{SETUP_COPY[reason].actionLabel}</Link>
            </Button>
        }
    />
)
