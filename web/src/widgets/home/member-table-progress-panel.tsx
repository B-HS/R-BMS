'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardQueryState } from '@features/dashboard/dashboard-query-state'
import { TableLevelProgressList } from '@features/dashboard/table-level-progress-list'
import { LampDistribution } from '@features/lamp/lamp-distribution'
import { useDashboardTableProgress } from '@entities/dashboard.query'
import { DISPLAY_LAMPS, isSameDisplayLamp } from '@shared/constants/clear-lamp'
import { DASHBOARD_PROGRESS_TABLES, DASHBOARD_WIDE_PANEL_CLASS_NAME } from '@shared/constants/dashboard'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { ToggleGroup, ToggleGroupItem } from '@shared/ui/toggle-group'

type ProgressTableId = (typeof DASHBOARD_PROGRESS_TABLES)[number]['id']

export const MemberTableProgressPanel: FC = () => {
    const [tableId, setTableId] = useState<ProgressTableId>(DASHBOARD_PROGRESS_TABLES[0].id)
    const t = useTranslations('pages.dashboard.tableProgress')
    const { data, isPending, isError, isFetching, refetch } = useDashboardTableProgress(tableId)
    const legend = DISPLAY_LAMPS.toReversed().map((displayed) => ({
        ...displayed,
        count: data?.lamps.find((entry) => isSameDisplayLamp(displayed, { lamp: entry.clear, assistGrade: entry.assist_grade }))?.count ?? 0,
    }))

    const handleTableChange = (value: string) => {
        const next = DASHBOARD_PROGRESS_TABLES.find((table) => table.id === value)
        if (next) setTableId(next.id)
    }

    return (
        <DashboardPanel
            title={t('title')}
            className={DASHBOARD_WIDE_PANEL_CLASS_NAME}
            badge={
                data && (
                    <span className='text-muted-foreground text-xs tabular-nums'>
                        <span className='text-foreground font-medium'>{formatCount(data.played)}</span> / {formatCount(data.total)} ·{' '}
                        {data.total === 0 ? 0 : Math.round((data.played / data.total) * PERCENT_SCALE)}%
                    </span>
                )
            }
            actions={
                <ToggleGroup
                    type='single'
                    variant='outline'
                    size='sm'
                    spacing={0}
                    aria-label={t('tableLabel')}
                    value={tableId}
                    onValueChange={handleTableChange}>
                    {DASHBOARD_PROGRESS_TABLES.map((table) => (
                        <ToggleGroupItem key={table.id} value={table.id} className='px-2 text-xs'>
                            {t(`tables.${table.labelKey}`)}
                        </ToggleGroupItem>
                    ))}
                </ToggleGroup>
            }>
            <DashboardQueryState
                isPending={isPending}
                isError={isError}
                isFetching={isFetching}
                onRetry={() => void refetch()}
                loadingLabel={t('loading')}
                errorMessage={t('loadError')}
                isEmpty={data?.total === 0}
                emptyMessage={t('empty')}>
                {data && (
                    <div className='@container'>
                        <div className='flex min-w-0 flex-col gap-3 @xl:flex-row @xl:gap-6'>
                            <div className='min-w-0 @xl:w-72 @xl:shrink-0'>
                                <LampDistribution lamps={legend} played={data.played} total={data.total} playedLabel={t('playedLabel')} />
                            </div>
                            <div className='min-w-0 flex-1'>
                                <TableLevelProgressList
                                    label={t('levelsLabel')}
                                    columns={{
                                        level: t('columnLevel'),
                                        lamps: t('columnLamps'),
                                        played: t('columnPlayed'),
                                        cleared: t('columnCleared'),
                                    }}
                                    levels={data.levels.map((level) => ({
                                        key: level.level,
                                        label: `${data.symbol}${level.level}`,
                                        lamps: level.lamps.map((entry) => ({
                                            lamp: entry.clear,
                                            assistGrade: entry.assist_grade,
                                            count: entry.count,
                                        })),
                                        played: level.played,
                                        total: level.total,
                                        cleared: level.cleared,
                                    }))}
                                />
                            </div>
                        </div>
                    </div>
                )}
            </DashboardQueryState>
        </DashboardPanel>
    )
}
