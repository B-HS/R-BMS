import type { FC, ReactNode } from 'react'
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle } from '@shared/ui/empty'
import { cn } from '@shared/lib/cn'

type EmptyStateProps = {
    title: string
    description?: string
    eyebrow?: string
    action?: ReactNode
    className?: string
}

export const EmptyState: FC<EmptyStateProps> = ({ title, description, eyebrow, action, className }) => (
    <Empty className={cn('bg-card min-h-48 gap-3 rounded-none border-0 p-6', className)}>
        <EmptyHeader className='gap-1'>
            {eyebrow && <p className='rbms-micro-label'>{eyebrow}</p>}
            <EmptyTitle>{title}</EmptyTitle>
            {description && <EmptyDescription className='text-xs'>{description}</EmptyDescription>}
        </EmptyHeader>
        {action && <EmptyContent>{action}</EmptyContent>}
    </Empty>
)
