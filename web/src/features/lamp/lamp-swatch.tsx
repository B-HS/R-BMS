import type { FC } from 'react'
import { lampBackground, type AssistGrade, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

type LampSwatchProps = {
    lamp: ClearLamp
    assistGrade?: AssistGrade
    className?: string
}

export const LampSwatch: FC<LampSwatchProps> = ({ lamp, assistGrade, className }) => (
    <span
        aria-hidden='true'
        className={cn('border-foreground/40 inline-block size-2.5 shrink-0 border', className)}
        style={{ background: lampBackground(lamp, assistGrade) }}
    />
)
