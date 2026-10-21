import type { FC } from 'react'
import { lampBackground, type AssistGrade, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

type LampStripProps = {
    lamp: ClearLamp
    assistGrade?: AssistGrade
    className?: string
}

export const LampStrip: FC<LampStripProps> = ({ lamp, assistGrade, className }) => (
    <span
        aria-hidden='true'
        data-lamp={lamp}
        className={cn('rbms-lamp w-1.25 shrink-0 self-stretch', className)}
        style={{ background: lampBackground(lamp, assistGrade) }}
    />
)
