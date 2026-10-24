import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import type { DisplayedDan } from '@entities/dan.type'

const ASSISTED_BADGE_CLASS = 'border-muted-foreground border-dashed'

export const DanBadge: FC<{ dan: DisplayedDan }> = ({ dan }) => {
    const t = useTranslations('pages.dan')

    return (
        <Badge
            variant={dan.assisted ? 'outline' : 'secondary'}
            className={dan.assisted ? ASSISTED_BADGE_CLASS : undefined}
            title={dan.assisted ? t('assisted') : undefined}>
            {dan.name} {dan.label}
        </Badge>
    )
}
