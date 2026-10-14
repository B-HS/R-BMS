import type { FC } from 'react'
import { LAMP_BACKGROUND, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

type LampSwatchProps = {
    lamp: ClearLamp
    className?: string
}

export const LampSwatch: FC<LampSwatchProps> = ({ lamp, className }) => (
    <span
        aria-hidden='true'
        className={cn('border-foreground/40 inline-block size-2.5 shrink-0 border', className)}
        style={{ background: LAMP_BACKGROUND[lamp] }}
    />
)
