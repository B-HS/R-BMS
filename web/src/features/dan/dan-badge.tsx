import type { FC } from 'react'
import { Badge } from '@shared/ui/badge'
import type { DisplayedDan } from '@entities/dan.type'

export const DanBadge: FC<{ dan: DisplayedDan }> = ({ dan }) => (
    <Badge variant='secondary'>
        {dan.name} {dan.label}
    </Badge>
)
