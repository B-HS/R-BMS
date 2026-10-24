'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { AiLimitsForm } from '@widgets/admin/ai-limits-form'
import { useAiOperatorLimits } from '@entities/ai.query'

export const AiLimitsPanel: FC = () => {
    const t = useTranslations('browse.admin.aiLimits')
    const { data, isLoading, isError, isFetching, refetch } = useAiOperatorLimits()

    return (
        <PanelCard
            title={t('title')}
            action={
                <Link href='/settings' className='text-muted-foreground text-xs underline'>
                    {t('toSettings')}
                </Link>
            }
            contentClassName='flex flex-col gap-3'>
            {isLoading && <TableSkeleton rows={3} label={t('loading')} />}
            {!isLoading && isError && <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />}
            {data && <AiLimitsForm limits={data} />}
            <PanelFootnote>{t('footnote')}</PanelFootnote>
        </PanelCard>
    )
}
