import type { FC } from 'react'
import { TriangleAlert } from 'lucide-react'
import { Button } from '@shared/ui/button'

type DashboardPanelErrorProps = {
    message: string
    isRetrying?: boolean
    onRetry?: () => void
}

export const DashboardPanelError: FC<DashboardPanelErrorProps> = ({ message, isRetrying = false, onRetry }) => (
    <div className='flex min-h-24 min-w-0 flex-1 flex-col items-center justify-center gap-2 p-3 text-center'>
        <TriangleAlert aria-hidden='true' className='text-destructive size-4 shrink-0' />
        <p role='alert' className='text-muted-foreground text-xs text-balance'>
            {message}
        </p>
        {onRetry && (
            <Button variant='outline' size='xs' disabled={isRetrying} onClick={onRetry}>
                다시 시도
            </Button>
        )}
    </div>
)
