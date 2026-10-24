import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import type { AiInsight } from '@entities/ai.type'

export const AiInsightList: FC<{ insights: readonly AiInsight[]; label: string }> = ({ insights, label }) => {
    const t = useTranslations('pages.ai.insightTone')

    return (
        <ul aria-label={label} className='flex flex-col gap-2'>
            {insights.map((insight) => (
                <li key={`${insight.key}-${insight.title}`} className='border-border flex flex-col gap-1 border p-2'>
                    <span className='flex flex-wrap items-center gap-2 text-sm font-medium'>
                        <Badge variant={insight.tone === 'weakness' ? 'destructive' : 'outline'}>{t(insight.tone)}</Badge>
                        {insight.title}
                    </span>
                    <p className='text-muted-foreground text-xs leading-5 break-keep'>{insight.body}</p>
                </li>
            ))}
        </ul>
    )
}
