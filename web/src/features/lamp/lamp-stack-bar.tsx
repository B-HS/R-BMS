import type { FC } from 'react'
import { CLEAR_LAMP_LABEL, LAMP_BACKGROUND, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

export type LampCount = {
    lamp: ClearLamp
    count: number
}

type LampStackBarProps = {
    lamps: LampCount[]
    className?: string
}

export const LampStackBar: FC<LampStackBarProps> = ({ lamps, className }) => (
    <div
        role='img'
        aria-label={lamps.map(({ lamp, count }) => `${CLEAR_LAMP_LABEL[lamp]} ${formatCount(count)}`).join(', ')}
        className={cn('bg-foreground/40 flex h-3 min-w-0 gap-px overflow-hidden p-px', className)}>
        {lamps
            .filter(({ count }) => count > 0)
            .map(({ lamp, count }) => (
                <span key={lamp} className='min-w-0.5 basis-0' style={{ flexGrow: count, background: LAMP_BACKGROUND[lamp] }} />
            ))}
    </div>
)
