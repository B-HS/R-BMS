'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { useTranslations } from 'next-intl'
import { ScratchAutoToggle } from '@features/filter/scratch-auto-toggle'
import { ScratchAutoBadge } from '@features/score/scratch-auto-badge'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { CourseSourceNotice } from '@features/course/course-source-notice'
import { LampLabel } from '@features/lamp/lamp-label'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { NotFoundNotice } from '@features/state/not-found-notice'
import { TableSkeleton } from '@features/state/table-skeleton'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { ApiError } from '@shared/lib/fetch'
import { formatCount, formatDateTime, formatHash } from '@shared/lib/format'
import { writeIncludeScratchAutoParam } from '@shared/lib/scratch-auto-param'
import { serializeSearchParams } from '@shared/lib/search-params'
import { DAN_SETTINGS_HREF } from '@shared/constants/navigation'
import { INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'
import { useCourseCatalogEntry, useCourseMeta, useCourseRanking } from '@entities/course.query'
import type { CourseChartRef, CourseRankingParams } from '@entities/course.type'
import type { ScoreRecord } from '@entities/score.type'

type CourseTranslator = ReturnType<typeof useTranslations<'browse.course'>>

const buildChartColumns = (titles: readonly string[], t: CourseTranslator): DataTableColumn<CourseChartRef>[] => [
    { key: 'stage', label: 'STAGE', width: 80, align: 'right', cell: (_row, index) => index + 1 },
    {
        key: 'chart',
        label: t('detail.chart'),
        flex: true,
        cell: (row, index) => (
            <Link href={`/charts/${row.sha256 || row.md5}`} className='hover:underline'>
                {titles[index] || <span className='font-mono'>{formatHash(row.sha256 || row.md5)}</span>}
            </Link>
        ),
    },
    { key: 'sha256', label: 'sha256', width: 112, mono: true, hideBelow: 'sm', cell: (row) => (row.sha256 ? formatHash(row.sha256) : '—') },
    { key: 'md5', label: 'md5', width: 112, mono: true, hideBelow: 'sm', cell: (row) => formatHash(row.md5) },
]

const buildRankingColumns = (t: CourseTranslator): DataTableColumn<ScoreRecord>[] => [
    { key: 'rank', label: t('detail.rank'), width: 64, align: 'right', cell: (row) => row.rank ?? '—' },
    {
        key: 'player_name',
        label: t('detail.player'),
        flex: true,
        cell: (row) => (
            <span className='flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5'>
                <Link href={`/players/${encodeURIComponent(row.player.id)}`} className='max-w-full truncate font-medium hover:underline'>
                    {row.player_name}
                </Link>
                {row.extra.scratch_auto === true && <ScratchAutoBadge />}
            </span>
        ),
    },
    { key: 'clear', label: t('detail.lamp'), width: 144, cell: (row) => <LampLabel lamp={row.clear} assistGrade={row.assist_grade} /> },
    { key: 'ex_score', label: 'EX', width: 96, align: 'right', cell: (row) => formatCount(row.ex_score) },
    { key: 'minbp', label: 'BP', width: 80, align: 'right', cell: (row) => formatCount(row.minbp) },
    { key: 'played_at', label: t('detail.recorded'), width: 144, align: 'right', cell: (row) => formatDateTime(row.played_at) },
]

const NOT_FOUND_STATUS = 404

type CourseDetailPanelProps = {
    courseHash: string
    rankingParams: CourseRankingParams
}

export const CourseDetailPanel: FC<CourseDetailPanelProps> = ({ courseHash, rankingParams }) => {
    const router = useRouter()
    const pathname = usePathname()
    const t = useTranslations('browse.course')
    const meta = useCourseMeta(courseHash)
    const ranking = useCourseRanking(courseHash, rankingParams)
    const catalogEntry = useCourseCatalogEntry(courseHash).data

    const rows = ranking.data ?? []
    const chartColumns = buildChartColumns(catalogEntry?.charts.map((chart) => chart.title) ?? [], t)

    const handleScratchAutoChange = (checked: boolean) => {
        const query = serializeSearchParams({ [INCLUDE_SCRATCH_AUTO_PARAM]: writeIncludeScratchAutoParam(checked) })
        router.push(query ? `${pathname}?${query}` : pathname, { scroll: false })
    }

    if (meta.isLoading) return <ShellPageLoading title={t('pageTitle')} />

    if (meta.error instanceof ApiError && meta.error.status === NOT_FOUND_STATUS)
        return (
            <ShellPage title={t('pageTitle')}>
                <NotFoundNotice
                    title={t('detail.notFoundTitle')}
                    description={t('detail.notFoundDescription')}
                    actionHref='/tables'
                    actionLabel={t('detail.notFoundAction')}
                />
            </ShellPage>
        )

    if (meta.isError || !meta.data)
        return (
            <ShellPage title={t('pageTitle')}>
                <LoadErrorState title={t('loadError')} isRetrying={meta.isFetching} onRetry={() => void meta.refetch()} />
            </ShellPage>
        )

    return (
        <ShellPage title={meta.data.name || t('pageTitle')} description={t('detail.chartCount', { count: meta.data.charts.length })}>
            <PanelCard title={t('pageTitle')} contentClassName='flex flex-col gap-2'>
                <div className='flex flex-wrap gap-1'>
                    <MetaBadge>lntype {meta.data.lntype}</MetaBadge>
                    {meta.data.constraint.map((value) => (
                        <MetaBadge key={value}>{value}</MetaBadge>
                    ))}
                </div>
                <p className='text-muted-foreground font-mono text-xs'>{meta.data.course_hash}</p>
                {catalogEntry && (
                    <>
                        <div className='flex flex-wrap gap-1'>
                            <MetaBadge>{catalogEntry.group.name}</MetaBadge>
                            {!catalogEntry.active && <MetaBadge>{t('removedFromTable')}</MetaBadge>}
                        </div>
                        {catalogEntry.dan && (
                            <p className='text-xs'>
                                {t.rich('detail.danAward', {
                                    system: catalogEntry.dan.system_name,
                                    label: catalogEntry.dan.label,
                                    link: (chunks) => (
                                        <Link href={DAN_SETTINGS_HREF} className='text-muted-foreground underline'>
                                            {chunks}
                                        </Link>
                                    ),
                                })}
                            </p>
                        )}
                        <CourseSourceNotice source={catalogEntry.source} keys={catalogEntry.group.keys} />
                    </>
                )}
            </PanelCard>
            <PanelCard title={t('detail.chartCount', { count: meta.data.charts.length })}>
                <DataTable
                    columns={chartColumns}
                    rows={meta.data.charts}
                    rowKey={(row) => `${row.sha256}-${row.md5}`}
                    caption={t('detail.chartsCaption')}
                />
            </PanelCard>
            {meta.data.trophy.length > 0 && (
                <PanelCard title={t('detail.trophies')} contentClassName='flex flex-col gap-1'>
                    {meta.data.trophy.map((trophy) => (
                        <p key={trophy.name} className='text-xs'>
                            <span className='font-medium'>{trophy.name}</span>
                            <span className='text-muted-foreground'>
                                {' '}
                                · scorerate {trophy.scorerate} · smissrate {trophy.smissrate}
                            </span>
                        </p>
                    ))}
                </PanelCard>
            )}
            <PanelCard
                title={t('detail.ranking')}
                action={<ScratchAutoToggle checked={rankingParams.include_scratch_auto} onCheckedChange={handleScratchAutoChange} />}
                contentClassName='flex flex-col gap-2'>
                {ranking.isLoading && <TableSkeleton />}
                {!ranking.isLoading && ranking.isError && (
                    <LoadErrorState title={t('detail.rankingLoadError')} isRetrying={ranking.isFetching} onRetry={() => void ranking.refetch()} />
                )}
                {!ranking.isLoading && !ranking.isError && rows.length === 0 && <EmptyState title={t('detail.noRecords')} />}
                {!ranking.isLoading && !ranking.isError && rows.length > 0 && (
                    <DataTable
                        columns={buildRankingColumns(t)}
                        rows={rows}
                        rowKey={(row) => `${row.player.id}-${row.played_at}`}
                        caption={t('detail.rankingCaption')}
                    />
                )}
                <PanelFootnote>{t('detail.footnote')}</PanelFootnote>
            </PanelCard>
        </ShellPage>
    )
}
