import type { FC } from 'react'
import { LampSwatch } from '@features/lamp/lamp-swatch'
import { CLEAR_LAMP_LABEL, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

type LampLabelProps = {
    lamp: ClearLamp
    className?: string
}

export const LampLabel: FC<LampLabelProps> = ({ lamp, className }) => (
    <span className={cn('text-foreground inline-flex min-w-0 items-center gap-1.5 whitespace-nowrap', className)}>
        <LampSwatch lamp={lamp} />
        <span className='truncate'>{CLEAR_LAMP_LABEL[lamp]}</span>
    </span>
)
