import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { SCRATCH_AUTO_BADGE_LABEL } from '@shared/constants/scratch-auto'
import { cn } from '@shared/lib/cn'
import { Badge } from '@shared/ui/badge'

export const ScratchAutoBadge: FC<{ className?: string }> = ({ className }) => {
    const t = useTranslations('pages.scratchAuto')
    const title = t('badgeTitle')

    return (
        <Badge variant='outline' title={title} className={cn('text-muted-foreground text-2xs h-4 rounded-none px-1', className)}>
            {SCRATCH_AUTO_BADGE_LABEL}
            <span className='sr-only'> ({title})</span>
        </Badge>
    )
}
