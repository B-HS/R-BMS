import type { FC } from 'react'
import { SCRATCH_AUTO_BADGE_LABEL, SCRATCH_AUTO_BADGE_TITLE } from '@shared/constants/scratch-auto'
import { cn } from '@shared/lib/cn'
import { Badge } from '@shared/ui/badge'

export const ScratchAutoBadge: FC<{ className?: string }> = ({ className }) => (
    <Badge variant='outline' title={SCRATCH_AUTO_BADGE_TITLE} className={cn('text-muted-foreground text-2xs h-4 rounded-none px-1', className)}>
        {SCRATCH_AUTO_BADGE_LABEL}
        <span className='sr-only'> ({SCRATCH_AUTO_BADGE_TITLE})</span>
    </Badge>
)
