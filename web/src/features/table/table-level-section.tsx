'use client'

import { useId, type FC, type PropsWithChildren } from 'react'
import { ChevronDown } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { LampStackBar, type LampCount } from '@features/lamp/lamp-stack-bar'
import { formatCount } from '@shared/lib/format'
import { Collapsible } from '@shared/ui/collapsible'
import { CollapsibleContent } from '@shared/ui/collapsible-content'
import { CollapsibleTrigger } from '@shared/ui/collapsible-trigger'
import { Skeleton } from '@shared/ui/skeleton'

type TableLevelSectionProps = PropsWithChildren<{
    anchorId: string
    label: string
    total: number
    shown: number
    lamps: LampCount[] | null
    played: number | null
    isRecordPending: boolean
    isOpen: boolean
    onOpenChange: (isOpen: boolean) => void
}>

export const TableLevelSection: FC<TableLevelSectionProps> = ({
    anchorId,
    label,
    total,
    shown,
    lamps,
    played,
    isRecordPending,
    isOpen,
    onOpenChange,
    children,
}) => {
    const t = useTranslations('pages.table.levelSection')
    const headingId = useId()
    const countText = shown === total ? t('count', { total }) : t('countShown', { shown, total })
    const progressText = played === null ? countText : `${formatCount(played)} / ${formatCount(total)}`
    const accessibleProgress = played === null ? countText : t('playedOfTotal', { played, total })

    return (
        <Collapsible open={isOpen} onOpenChange={onOpenChange} asChild>
            <section id={anchorId} aria-labelledby={headingId} className='min-w-0 max-md:-scroll-mt-(--rail-chrome-height)'>
                <h2 id={headingId} className='rbms-level-header bg-muted'>
                    <CollapsibleTrigger
                        aria-label={`${label} · ${accessibleProgress}${shown === total ? '' : ` · ${t('matching', { shown })}`}`}
                        className='group focus-visible:ring-ring flex h-12 w-full min-w-0 items-center gap-3 px-3 text-left outline-none focus-visible:ring-2 focus-visible:ring-inset'>
                        <ChevronDown aria-hidden className='size-4 shrink-0 transition-transform group-data-[state=closed]:-rotate-90' />
                        <span className='w-12 shrink-0 text-sm font-semibold tabular-nums'>{label}</span>
                        <span className='flex min-w-0 flex-1 items-center'>
                            {lamps && <LampStackBar lamps={lamps} className='w-full max-w-64' />}
                            {!lamps && isRecordPending && <Skeleton className='h-3 w-full max-w-64' />}
                        </span>
                        <span className='text-muted-foreground shrink-0 text-xs tabular-nums'>
                            {progressText}
                            {played !== null && shown !== total && <span className='text-foreground ml-2'>{t('condition', { shown })}</span>}
                        </span>
                    </CollapsibleTrigger>
                </h2>
                <CollapsibleContent>{children}</CollapsibleContent>
            </section>
        </Collapsible>
    )
}
