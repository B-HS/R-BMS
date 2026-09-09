import type { FC } from 'react'
import { Item, ItemDescription, ItemTitle } from '@shared/ui/item'
import { cn } from '@shared/lib/cn'

type StatTileProps = {
    label: string
    value: string
    hint?: string
    accent?: 'warning' | 'danger'
}

const ACCENT_CLASS = {
    warning: 'text-warning',
    danger: 'text-destructive',
}

export const StatTile: FC<StatTileProps> = ({ label, value, hint, accent }) => (
    <Item asChild className='bg-card h-full flex-col items-start gap-1 border-0 p-3'>
        <article>
            <ItemDescription className='text-xs'>{label}</ItemDescription>
            <ItemTitle className={cn('text-2xl font-semibold tracking-tight tabular-nums', accent && ACCENT_CLASS[accent])}>{value}</ItemTitle>
            {hint && <ItemDescription className='text-xs'>{hint}</ItemDescription>}
        </article>
    </Item>
)
