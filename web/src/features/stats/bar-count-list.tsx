import type { FC } from 'react'
import { formatCount } from '@shared/lib/format'

type BarCountListProps = {
    label: string
    items: { key: string; label: string; count: number }[]
}

export const BarCountList: FC<BarCountListProps> = ({ label, items }) => {
    const max = items.reduce((highest, item) => Math.max(highest, item.count), 0)
    return (
        <section className='flex flex-col gap-1.5'>
            <h3 className='text-muted-foreground text-2xs font-medium tracking-wide uppercase'>{label}</h3>
            {items.map((item) => (
                <div key={item.key} className='flex flex-col gap-1'>
                    <div className='flex items-center gap-2 text-xs'>
                        <span className='min-w-0 flex-1 overflow-hidden font-mono text-ellipsis whitespace-nowrap'>{item.label}</span>
                        <span className='text-muted-foreground shrink-0 tabular-nums'>{formatCount(item.count)}</span>
                    </div>
                    <div className='bg-border h-px w-full'>
                        <div
                            className='bg-foreground/40 ease-standard h-full transition-[width] duration-[var(--motion-bar-duration)]'
                            style={{ width: max === 0 ? '0%' : `${(item.count / max) * 100}%` }}
                        />
                    </div>
                </div>
            ))}
        </section>
    )
}
