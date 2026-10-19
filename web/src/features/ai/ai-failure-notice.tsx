import type { FC } from 'react'
import Link from 'next/link'
import { TriangleAlert } from 'lucide-react'
import { Alert, AlertDescription, AlertTitle } from '@shared/ui/alert'
import { Button } from '@shared/ui/button'
import { AI_SETTINGS_HREF } from '@shared/constants/navigation'
import { AI_ERROR_CODE, toAiFailureView } from '@entities/ai.type'

export const AiFailureNotice: FC<{ error: unknown }> = ({ error }) => {
    const failure = toAiFailureView(error)

    return (
        <Alert role='alert' variant='destructive'>
            <TriangleAlert aria-hidden />
            <AlertTitle>AI 호출이 실패했습니다</AlertTitle>
            <AlertDescription className='flex flex-col gap-2 break-keep'>
                <p className='text-foreground'>{failure.message}</p>
                {failure.retryAfterSeconds !== null && <p>공급자가 {failure.retryAfterSeconds}초 뒤에 다시 시도하라고 알려 왔습니다.</p>}
                {failure.code === AI_ERROR_CODE.MODEL_UNAVAILABLE && <p>설정에서 모델을 다시 고르세요.</p>}
                {failure.needsSettings && (
                    <div>
                        <Button asChild variant='outline' size='sm'>
                            <Link href={AI_SETTINGS_HREF}>AI 설정 열기</Link>
                        </Button>
                    </div>
                )}
            </AlertDescription>
        </Alert>
    )
}
