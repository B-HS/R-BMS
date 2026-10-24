'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { PanelCard } from '@features/shell/panel-card'
import { DanSystemCard } from '@widgets/settings/dan-system-card'
import { useMyDans } from '@entities/dan.query'

const SKELETON_ROWS = 4

const ELIGIBILITY_RULES = ['cleared', 'stages', 'constraints', 'exclusions', 'assisted', 'stageRecords', 'gaugeMode'] as const

export const DanPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const t = useTranslations('settings.danPanel')
    const { data, isLoading, isError, isFetching, refetch } = useMyDans()

    if (isLoading) return <TableSkeleton rows={SKELETON_ROWS} label={t('loading')} />
    if (isError || !data) return <LoadErrorState title={t('loadError')} isRetrying={isFetching} onRetry={() => void refetch()} />

    const hasAcquired = data.systems.some((system) => system.acquired.length > 0)

    return (
        <>
            <PanelCard title={t('title')} contentClassName='flex flex-col gap-2'>
                <p className='text-xs'>{t('intro')}</p>
                {!hasAcquired && (
                    <p className='text-xs font-medium'>
                        {t.rich('noneAcquired', {
                            link: (chunks) => (
                                <Link href='/courses' className='underline'>
                                    {chunks}
                                </Link>
                            ),
                        })}
                    </p>
                )}
                <ul aria-label={t('rulesLabel')} className='text-muted-foreground flex list-disc flex-col gap-0.5 pl-4 text-xs'>
                    {ELIGIBILITY_RULES.map((rule) => (
                        <li key={rule}>{t(`rules.${rule}`)}</li>
                    ))}
                </ul>
            </PanelCard>
            {data.systems.map((system) => (
                <DanSystemCard key={system.system} system={system} playerId={playerId} />
            ))}
        </>
    )
}
