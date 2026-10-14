import type { FC, PropsWithChildren, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'

type FilterBarProps = PropsWithChildren<{
    label: string
    summary?: ReactNode
    className?: string
}>

export const FilterBar: FC<FilterBarProps> = ({ label, summary, className, children }) => (
    <div
        role='group'
        aria-label={label}
        className={cn('border-border bg-background flex min-h-12 shrink-0 flex-wrap items-center gap-2 border-b px-3 py-2', className)}>
        {children}
        {summary && <div className='text-muted-foreground ml-auto shrink-0 text-xs tabular-nums'>{summary}</div>}
    </div>
)
