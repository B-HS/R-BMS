import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Progress } from '@shared/ui/progress'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

type AiUsageMeterProps = {
    label: string
    used: number
    limit: number | null
    unit: string
}

export const AiUsageMeter: FC<AiUsageMeterProps> = ({ label, used, limit, unit }) => {
    const t = useTranslations('pages.ai.usageMeter')

    return (
        <div className='flex flex-col gap-1'>
            <div className='flex items-baseline justify-between gap-2 text-xs'>
                <span className='text-muted-foreground'>{label}</span>
                <span className={cn('tabular-nums', limit !== null && used >= limit && 'text-warning')}>
                    {limit === null ? t('unlimited', { used, unit }) : `${formatCount(used)} / ${formatCount(limit)} ${unit}`}
                </span>
            </div>
            {limit !== null && <Progress value={Math.min(PERCENT_SCALE, (used / limit) * PERCENT_SCALE)} aria-label={t('rate', { label })} />}
        </div>
    )
}
