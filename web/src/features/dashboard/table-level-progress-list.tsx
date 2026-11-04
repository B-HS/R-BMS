import type { FC } from 'react'
import { LampStackBar, type LampCount } from '@features/lamp/lamp-stack-bar'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

type TableLevelProgressListLevel = {
    key: string
    label: string
    lamps: LampCount[]
    played: number
    total: number
    cleared: number
}

type TableLevelProgressListProps = {
    levels: readonly TableLevelProgressListLevel[]
    label: string
    columns: { level: string; lamps: string; played: string; cleared: string }
}

const ROW_CLASS_NAME = 'grid grid-cols-[3rem_minmax(0,1fr)_4.5rem_3rem] items-center gap-2 px-1 text-xs'

export const TableLevelProgressList: FC<TableLevelProgressListProps> = ({ levels, label, columns }) => (
    <div className='flex min-w-0 flex-col'>
        <div aria-hidden='true' className={cn(ROW_CLASS_NAME, 'text-muted-foreground text-2xs h-5')}>
            <span>{columns.level}</span>
            <span>{columns.lamps}</span>
            <span className='text-right'>{columns.played}</span>
            <span className='text-right'>{columns.cleared}</span>
        </div>
        <ul aria-label={label} className='flex max-h-96 flex-col overflow-y-auto'>
            {levels.map((level) => (
                <li key={level.key} className={cn(ROW_CLASS_NAME, 'h-6 shrink-0')}>
                    <span className='truncate font-medium tabular-nums'>{level.label}</span>
                    <LampStackBar lamps={level.lamps} className='h-2.5' />
                    <span className='text-right tabular-nums'>
                        {formatCount(level.played)}/{formatCount(level.total)}
                    </span>
                    <span className='text-muted-foreground text-right tabular-nums'>{formatCount(level.cleared)}</span>
                </li>
            ))}
        </ul>
    </div>
)
