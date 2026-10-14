import type { FC } from 'react'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

type BarCountListProps = {
    label: string
    items: { key: string; label: string; count: number; color?: string }[]
}

export const BarCountList: FC<BarCountListProps> = ({ label, items }) => {
    const max = items.reduce((highest, item) => Math.max(highest, item.count), 0)

    return (
        <section className='flex flex-col gap-1.5'>
            <h3 className='rbms-micro-label'>{label}</h3>
            <ul className='flex flex-col gap-1'>
                {items.map((item) => (
                    <li key={item.key} className='grid grid-cols-[7rem_minmax(0,1fr)_3.5rem] items-center gap-2 text-xs'>
                        <span className='truncate'>{item.label}</span>
                        <span className='bg-muted h-2 min-w-0'>
                            <span
                                className='ease-standard block h-full transition-[width] duration-(--motion-bar-duration)'
                                style={{
                                    width: max === 0 ? '0%' : `${(item.count / max) * PERCENT_SCALE}%`,
                                    background: item.color ?? 'color-mix(in oklch, var(--foreground) 40%, transparent)',
                                }}
                            />
                        </span>
                        <span className={cn('text-right tabular-nums', item.count === 0 ? 'text-muted-foreground' : 'font-medium')}>
                            {formatCount(item.count)}
                        </span>
                    </li>
                ))}
            </ul>
        </section>
    )
}
