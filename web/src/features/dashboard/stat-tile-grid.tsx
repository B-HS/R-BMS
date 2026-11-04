import type { FC } from 'react'

type StatTileGridItem = {
    key: string
    label: string
    value: string
    hint?: string
}

type StatTileGridProps = {
    label: string
    items: readonly StatTileGridItem[]
}

export const StatTileGrid: FC<StatTileGridProps> = ({ label, items }) => (
    <div className='@container'>
        <dl aria-label={label} className='bg-border grid grid-cols-2 gap-px @md:grid-cols-4'>
            {items.map((item) => (
                <div key={item.key} className='bg-card flex min-w-0 flex-col gap-0.5 p-3'>
                    <dt className='text-muted-foreground text-xs break-keep'>{item.label}</dt>
                    <dd className='text-xl leading-7 font-semibold tracking-tight tabular-nums'>{item.value}</dd>
                    {item.hint && <dd className='text-muted-foreground text-2xs'>{item.hint}</dd>}
                </div>
            ))}
        </dl>
    </div>
)
