import type { FC } from 'react'
import { cn } from '@shared/lib/cn'

type TableLevelJumpItem = {
    anchorId: string
    label: string
    isAvailable: boolean
}

type TableLevelJumpProps = {
    items: TableLevelJumpItem[]
    onJump: (anchorId: string) => void
    className?: string
}

export const TableLevelJump: FC<TableLevelJumpProps> = ({ items, onJump, className }) => (
    <nav aria-label='레벨 이동' className={cn('bg-background min-w-0', className)}>
        <ul className='flex min-w-0 gap-1 overflow-x-auto px-3 py-2 md:flex-wrap md:overflow-visible'>
            {items.map((item) => (
                <li key={item.anchorId} className='shrink-0'>
                    {item.isAvailable ? (
                        <a
                            href={`#${item.anchorId}`}
                            onClick={() => onJump(item.anchorId)}
                            className='border-border hover:bg-muted focus-visible:ring-ring flex h-7 min-w-10 items-center justify-center border px-2 text-xs font-medium md:min-w-9 md:px-1.5 tabular-nums outline-none focus-visible:ring-2'>
                            {item.label}
                        </a>
                    ) : (
                        <span
                            aria-disabled='true'
                            className='border-border text-muted-foreground/60 flex h-7 min-w-10 items-center justify-center border border-dashed px-2 text-xs md:min-w-9 md:px-1.5 tabular-nums'>
                            {item.label}
                        </span>
                    )}
                </li>
            ))}
        </ul>
    </nav>
)
