import type { FC } from 'react'
import { lampBackground, lampKey, lampLabel, type AssistGrade, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

export type LampCount = {
    lamp: ClearLamp
    assistGrade?: AssistGrade
    count: number
}

type LampStackBarProps = {
    lamps: LampCount[]
    className?: string
}

export const LampStackBar: FC<LampStackBarProps> = ({ lamps, className }) => (
    <div
        role='img'
        aria-label={lamps.map(({ lamp, assistGrade, count }) => `${lampLabel(lamp, assistGrade)} ${formatCount(count)}`).join(', ')}
        className={cn('bg-foreground/40 flex h-3 min-w-0 gap-px overflow-hidden p-px', className)}>
        {lamps
            .filter(({ count }) => count > 0)
            .map(({ lamp, assistGrade, count }) => (
                <span
                    key={lampKey(lamp, assistGrade)}
                    className='min-w-0.5 basis-0'
                    style={{ flexGrow: count, background: lampBackground(lamp, assistGrade) }}
                />
            ))}
    </div>
)
