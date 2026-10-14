import type { FC } from 'react'
import { LAMP_BACKGROUND, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

type LampStripProps = {
    lamp: ClearLamp
    className?: string
}

export const LampStrip: FC<LampStripProps> = ({ lamp, className }) => (
    <span
        aria-hidden='true'
        data-lamp={lamp}
        className={cn('rbms-lamp w-1.25 shrink-0 self-stretch', className)}
        style={{ background: LAMP_BACKGROUND[lamp] }}
    />
)
