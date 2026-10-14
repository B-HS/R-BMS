import type { FC } from 'react'
import { LampStackBar, type LampCount } from '@features/lamp/lamp-stack-bar'
import { LampSwatch } from '@features/lamp/lamp-swatch'
import { CLEAR_LAMP_LABEL } from '@shared/constants/clear-lamp'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

type LampDistributionProps = {
    lamps: LampCount[]
    played: number
    total: number
    playedLabel?: string
    rateLabel?: string
}

export const LampDistribution: FC<LampDistributionProps> = ({ lamps, played, total, playedLabel = '플레이한 차트', rateLabel = '플레이 비율' }) => (
    <div className='grid min-w-0 shrink-0 gap-2'>
        <dl className='flex min-w-0 items-end justify-between gap-3'>
            <div className='flex min-w-0 items-baseline gap-2'>
                <dt className='text-muted-foreground shrink-0 text-xs'>{playedLabel}</dt>
                <dd className='min-w-0 truncate tabular-nums'>
                    <span className='text-xl leading-7 font-semibold tracking-tight'>{formatCount(played)}</span>
                    <span className='text-muted-foreground text-xs'> / {formatCount(total)}</span>
                </dd>
            </div>
            <div className='flex shrink-0 items-baseline gap-1.5'>
                <dt className='text-muted-foreground text-xs'>{rateLabel}</dt>
                <dd className='text-sm font-semibold tabular-nums'>{total === 0 ? 0 : Math.round((played / total) * PERCENT_SCALE)}%</dd>
            </div>
        </dl>
        <LampStackBar lamps={lamps} />
        <ul className='grid min-w-0 grid-cols-2 gap-x-4 gap-y-0.5 text-xs'>
            {lamps.map(({ lamp, count }) => (
                <li key={lamp} className='flex min-w-0 items-center justify-between gap-2'>
                    <span className='flex min-w-0 items-center gap-1.5'>
                        <LampSwatch lamp={lamp} />
                        <span className='truncate'>{CLEAR_LAMP_LABEL[lamp]}</span>
                    </span>
                    <span className={cn('shrink-0 tabular-nums', count === 0 ? 'text-muted-foreground' : 'font-medium')}>{formatCount(count)}</span>
                </li>
            ))}
        </ul>
    </div>
)
