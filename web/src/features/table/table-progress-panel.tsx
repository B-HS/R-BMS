'use client'

import { useId, useState, type FC } from 'react'
import { ChevronDown } from 'lucide-react'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { LampDistribution } from '@features/lamp/lamp-distribution'
import { LampStackBar, type LampCount } from '@features/lamp/lamp-stack-bar'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

type TableProgressLevel = {
    key: string
    label: string
    lamps: LampCount[]
    played: number
    total: number
}

type TableProgressPanelProps = {
    lamps: LampCount[]
    played: number
    total: number
    levels: TableProgressLevel[]
}

export const TableProgressPanel: FC<TableProgressPanelProps> = ({ lamps, played, total, levels }) => {
    const [isDetailOpen, setIsDetailOpen] = useState(false)
    const detailId = useId()

    return (
        <DashboardPanel
            title='내 진행'
            badge={
                <span className='text-muted-foreground text-xs tabular-nums'>
                    <span className='text-foreground font-medium'>{formatCount(played)}</span> / {formatCount(total)} ·{' '}
                    {total === 0 ? 0 : Math.round((played / total) * PERCENT_SCALE)}%
                </span>
            }
            actions={
                <Button
                    type='button'
                    variant='ghost'
                    size='xs'
                    aria-expanded={isDetailOpen}
                    aria-controls={detailId}
                    onClick={() => setIsDetailOpen((isOpen) => !isOpen)}>
                    {isDetailOpen ? '접기' : '레벨별 보기'}
                    <ChevronDown aria-hidden className={cn('transition-transform', !isDetailOpen && '-rotate-90')} />
                </Button>
            }
            bodyClassName='gap-3'>
            {!isDetailOpen && <LampStackBar lamps={lamps} />}
            <div id={detailId} hidden={!isDetailOpen} className='flex flex-col gap-3 md:flex-row md:gap-6'>
                <div className='md:w-72 md:shrink-0'>
                    <LampDistribution lamps={lamps} played={played} total={total} playedLabel='플레이한 곡' rateLabel='플레이 비율' />
                </div>
                <ul aria-label='레벨별 램프 분포' className='grid min-w-0 flex-1 grid-cols-1 gap-x-4 gap-y-0.5 sm:grid-cols-2 xl:grid-cols-3'>
                    {levels.map((level) => (
                        <li key={level.key} className='flex h-6 min-w-0 items-center gap-2 px-1 text-xs'>
                            <span className='w-10 shrink-0 font-medium tabular-nums'>{level.label}</span>
                            <LampStackBar lamps={level.lamps} className='h-2.5 flex-1' />
                            <span className='text-muted-foreground w-14 shrink-0 text-right tabular-nums'>
                                {formatCount(level.played)}/{formatCount(level.total)}
                            </span>
                        </li>
                    ))}
                </ul>
            </div>
        </DashboardPanel>
    )
}
