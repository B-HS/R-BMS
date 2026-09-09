'use client'

import type { FC } from 'react'
import { cn } from '@shared/lib/cn'

type ColumnToggleProps = {
    options: { key: string; label: string }[]
    selected: string[]
    onToggle: (key: string) => void
}

export const ColumnToggle: FC<ColumnToggleProps> = ({ options, selected, onToggle }) => (
    <div className='flex flex-wrap items-center gap-1'>
        {options.map((option) => (
            <button
                key={option.key}
                type='button'
                aria-pressed={selected.includes(option.key)}
                onClick={() => onToggle(option.key)}
                className={cn(
                    'border-border h-7 border px-2 text-xs font-medium transition-opacity',
                    selected.includes(option.key) ? 'bg-accent text-accent-foreground' : 'text-muted-foreground',
                )}>
                {option.label}
            </button>
        ))}
    </div>
)
