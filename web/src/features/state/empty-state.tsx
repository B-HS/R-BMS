import type { FC } from 'react'
import { Inbox } from 'lucide-react'
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@shared/ui/empty'

export const EmptyState: FC<{ title: string; description?: string }> = ({ title, description }) => (
    <Empty className='bg-card gap-6 p-3'>
        <EmptyHeader className='gap-1'>
            <EmptyMedia>
                <Inbox className='size-6' aria-hidden />
            </EmptyMedia>
            <EmptyTitle>{title}</EmptyTitle>
            {description && <EmptyDescription className='text-xs'>{description}</EmptyDescription>}
        </EmptyHeader>
    </Empty>
)
