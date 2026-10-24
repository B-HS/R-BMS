import type { FC } from 'react'
import { TriangleAlert } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'
import { cn } from '@shared/lib/cn'

type LoadErrorStateProps = {
    title: string
    description?: string
    isRetrying?: boolean
    onRetry?: () => void
    className?: string
}

export const LoadErrorState: FC<LoadErrorStateProps> = ({ title, description, isRetrying = false, onRetry, className }) => {
    const t = useTranslations('pages.state')
    const tShared = useTranslations('pages.shared')

    return (
        <div className={cn('bg-card flex min-h-32 flex-col items-center justify-center gap-3 p-6 text-center text-balance', className)}>
            <TriangleAlert className='text-destructive size-4 shrink-0' aria-hidden />
            <div role='alert' className='flex flex-col gap-1'>
                <p className='text-sm font-medium'>{title}</p>
                <p className='text-muted-foreground text-xs'>{description ?? t('requestFailed')}</p>
            </div>
            {onRetry && (
                <Button variant='outline' size='sm' disabled={isRetrying} onClick={onRetry}>
                    {tShared('retry')}
                </Button>
            )}
        </div>
    )
}
