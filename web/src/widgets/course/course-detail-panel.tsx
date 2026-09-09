'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { PageHeading } from '@features/shell/page-heading'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { MetaBadge } from '@features/badge/meta-badge'
import { LampBadge } from '@features/badge/lamp-badge'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { TableSkeleton } from '@features/state/table-skeleton'
import { formatCount, formatDateTime, formatHash } from '@shared/lib/format'
import { useCourseMeta, useCourseRanking } from '@entities/course.query'
import type { CourseChartRef } from '@entities/course.type'
import type { ScoreRecord } from '@entities/score.type'

const CHART_COLUMNS: DataTableColumn<CourseChartRef>[] = [
    { key: 'stage', label: 'STAGE', width: 80, align: 'right', cell: (_row, index) => index + 1 },
    {
        key: 'sha256',
        label: '차트',
        flex: true,
        mono: true,
        cell: (row) => (
            <Link href={`/charts/${row.sha256}`} className='hover:underline'>
                {formatHash(row.sha256)}
            </Link>
        ),
    },
    { key: 'md5', label: 'md5', width: 128, mono: true, cell: (row) => formatHash(row.md5) },
]

const RANKING_COLUMNS: DataTableColumn<ScoreRecord>[] = [
    { key: 'rank', label: '순위', width: 64, align: 'right', cell: (row) => row.rank ?? '—' },
    { key: 'player_name', label: '플레이어', flex: true, cell: (row) => <Link href={`/players/${row.player.id}`}>{row.player_name}</Link> },
    { key: 'clear', label: '램프', width: 128, cell: (row) => <LampBadge clear={row.clear} /> },
    { key: 'ex_score', label: 'EX', width: 96, align: 'right', cell: (row) => formatCount(row.ex_score) },
    { key: 'minbp', label: 'BP', width: 80, align: 'right', cell: (row) => formatCount(row.minbp) },
    { key: 'played_at', label: '기록', width: 144, align: 'right', cell: (row) => formatDateTime(row.played_at) },
]

type CourseDetailPanelProps = {
    courseHash: string
    rankingParams: { limit: number }
}

export const CourseDetailPanel: FC<CourseDetailPanelProps> = ({ courseHash, rankingParams }) => {
    const meta = useCourseMeta(courseHash)
    const ranking = useCourseRanking(courseHash, rankingParams)

    const rows = ranking.data ?? []

    if (meta.isLoading) return <PanelSkeleton height='page' />
    if (meta.isError || !meta.data) return <LoadErrorState title='코스를 불러오지 못했습니다' />

    return (
        <>
            <PageHeading>{meta.data.name}</PageHeading>
            <PanelCard title='코스' contentClassName='flex flex-col gap-2'>
                <div className='flex flex-wrap gap-1'>
                    <MetaBadge>lntype {meta.data.lntype}</MetaBadge>
                    {meta.data.constraint.map((value) => (
                        <MetaBadge key={value}>{value}</MetaBadge>
                    ))}
                </div>
                <p className='text-muted-foreground font-mono text-xs'>{meta.data.course_hash}</p>
            </PanelCard>
            <PanelCard title={`구성 차트 ${formatCount(meta.data.charts.length)}곡`}>
                <DataTable columns={CHART_COLUMNS} rows={meta.data.charts} rowKey={(row) => `${row.sha256}-${row.md5}`} caption='코스 구성 차트' />
            </PanelCard>
            {meta.data.trophy.length > 0 && (
                <PanelCard title='트로피' contentClassName='flex flex-col gap-1'>
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
            <PanelCard title='랭킹' contentClassName='flex flex-col gap-2'>
                {ranking.isLoading && <TableSkeleton />}
                {!ranking.isLoading && ranking.isError && <LoadErrorState title='랭킹을 불러오지 못했습니다' />}
                {!ranking.isLoading && !ranking.isError && rows.length === 0 && <EmptyState title='기록이 없습니다' />}
                {!ranking.isLoading && !ranking.isError && rows.length > 0 && (
                    <DataTable columns={RANKING_COLUMNS} rows={rows} rowKey={(row) => `${row.player.id}-${row.played_at}`} caption='코스 랭킹' />
                )}
                <PanelFootnote>unranked 기록은 랭킹에서 제외됩니다.</PanelFootnote>
            </PanelCard>
        </>
    )
}
