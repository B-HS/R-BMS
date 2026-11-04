'use client'

import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiSetupNotice } from '@features/ai/ai-setup-notice'
import { PanelCard } from '@features/shell/panel-card'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { AiTrainingCreateForm } from '@widgets/ai/ai-training-create-form'
import { AiTrainingList } from '@widgets/ai/ai-training-list'
import { formatCount } from '@shared/lib/format'
import { useAiTrainingList } from '@entities/ai.query'

/**
 * The training chart tab (plan 2026-11-04 7.3): the room left of the 20 charts and the form that makes one (blocked without an AI
 * key, and replaced by a "delete one first" notice at the limit), and under it how to get the charts into the game and the list.
 * `initialFocusKey` opens the form on a weakness, as the profile's links do.
 */
export const AiTrainingPanel: FC<{ initialFocusKey?: string | null }> = ({ initialFocusKey = null }) => {
    const t = useTranslations('pages.ai.training')
    const list = useAiTrainingList()
    const view = list.data

    return (
        <>
            <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
                {list.isPending && <PanelSkeleton label={t('loading')} />}
                {list.isError && <LoadErrorState title={t('loadError')} isRetrying={list.isFetching} onRetry={() => void list.refetch()} />}
                {view && (
                    <>
                        <p className='text-xs tabular-nums'>{t('usage', { used: formatCount(view.used), limit: formatCount(view.limit) })}</p>
                        {view.needs_key && <AiSetupNotice reason='no_key' />}
                        {!view.needs_key && !view.can_create && (
                            <EmptyState title={t('fullTitle')} description={t('fullDescription', { limit: view.limit })} />
                        )}
                        {!view.needs_key && view.can_create && <AiTrainingCreateForm initialFocusKey={initialFocusKey} />}
                    </>
                )}
            </PanelCard>
            <PanelCard title={t('list.title')} contentClassName='flex flex-col gap-3'>
                <div className='text-muted-foreground flex flex-col gap-1 text-xs break-keep'>
                    <p>{t('guide.intro')}</p>
                    <p>{t('guide.steps')}</p>
                </div>
                {view && view.charts.length === 0 && <EmptyState title={t('list.emptyTitle')} description={t('list.emptyDescription')} />}
                {view && view.charts.length > 0 && <AiTrainingList charts={view.charts} />}
            </PanelCard>
        </>
    )
}
