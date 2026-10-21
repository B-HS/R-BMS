import type { FC } from 'react'
import { LampSwatch } from '@features/lamp/lamp-swatch'
import { lampLabel, type AssistGrade, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

type LampLabelProps = {
    lamp: ClearLamp
    assistGrade?: AssistGrade
    className?: string
}

export const LampLabel: FC<LampLabelProps> = ({ lamp, assistGrade, className }) => (
    <span className={cn('text-foreground inline-flex min-w-0 items-center gap-1.5 whitespace-nowrap', className)}>
        <LampSwatch lamp={lamp} assistGrade={assistGrade} />
        <span className='truncate'>{lampLabel(lamp, assistGrade)}</span>
    </span>
)
