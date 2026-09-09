import type { FC, PropsWithChildren, ReactNode } from 'react'
import { Card, CardAction, CardContent, CardHeader } from '@shared/ui/card'
import { cn } from '@shared/lib/cn'

type PanelCardProps = PropsWithChildren<{
    title?: ReactNode
    action?: ReactNode
    contentClassName?: string
    className?: string
}>

export const PanelCard: FC<PanelCardProps> = ({ title, action, children, contentClassName, className }) => (
    <Card size='sm' className={cn('bg-card ring-0', className)}>
        {title && (
            <CardHeader className='flex flex-wrap items-center justify-between gap-2'>
                <h2 className='text-card-foreground text-sm font-medium'>{title}</h2>
                {action && <CardAction className='row-start-auto self-auto'>{action}</CardAction>}
            </CardHeader>
        )}
        <CardContent className={contentClassName}>{children}</CardContent>
    </Card>
)
