import type { FC } from 'react'
import { TriangleAlert } from 'lucide-react'
import { Button } from '@shared/ui/button'
import { cn } from '@shared/lib/cn'

type LoadErrorStateProps = {
    title: string
    description?: string
    isRetrying?: boolean
    onRetry?: () => void
    className?: string
}

const DEFAULT_DESCRIPTION = '요청이 실패했습니다. 잠시 후 다시 시도하세요.'

export const LoadErrorState: FC<LoadErrorStateProps> = ({ title, description = DEFAULT_DESCRIPTION, isRetrying = false, onRetry, className }) => (
    <div className={cn('bg-card flex min-h-32 flex-col items-center justify-center gap-3 p-6 text-center text-balance', className)}>
        <TriangleAlert className='text-destructive size-4 shrink-0' aria-hidden />
        <div role='alert' className='flex flex-col gap-1'>
            <p className='text-sm font-medium'>{title}</p>
            <p className='text-muted-foreground text-xs'>{description}</p>
        </div>
        {onRetry && (
            <Button variant='outline' size='sm' disabled={isRetrying} onClick={onRetry}>
                다시 시도
            </Button>
        )}
    </div>
)
