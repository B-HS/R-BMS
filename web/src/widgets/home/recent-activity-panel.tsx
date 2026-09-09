'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { LampBadge } from '@features/badge/lamp-badge'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { useActivityRecent } from '@entities/stats.query'
import type { ActivityRow } from '@entities/stats.type'
import { formatCount, formatDateTime } from '@shared/lib/format'

const RECENT_LIMIT = 20

const COLUMNS: DataTableColumn<ActivityRow>[] = [
    { key: 'played_at', label: '시각', width: 160, mono: true, cell: (row) => formatDateTime(row.played_at) },
    {
        key: 'player',
        label: '플레이어',
        width: 112,
        cell: (row) => (
            <Link href={`/players/${row.player.id}`} className='hover:underline'>
                {row.player_name}
            </Link>
        ),
    },
    {
        key: 'title',
        label: '차트',
        flex: true,
        cell: (row) => (
            <Link href={`/charts/${row.chart.sha256}`} className='hover:underline'>
                {row.chart.title}
            </Link>
        ),
    },
    { key: 'level', label: '레벨', width: 64, align: 'right', cell: (row) => row.chart.level ?? '—' },
    { key: 'clear', label: '램프', width: 128, cell: (row) => <LampBadge clear={row.clear} /> },
    { key: 'ex_score', label: 'EX', width: 96, align: 'right', cell: (row) => formatCount(row.ex_score) },
    { key: 'minbp', label: 'BP', width: 64, align: 'right', cell: (row) => formatCount(row.minbp) },
]

export const RecentActivityPanel: FC = () => {
    const { data, isLoading, isError } = useActivityRecent({ limit: RECENT_LIMIT })

    return (
        <PanelCard title='최근 활동' contentClassName='flex flex-col gap-2'>
            {isLoading && <TableSkeleton />}
            {!isLoading && isError && <LoadErrorState title='최근 활동을 불러오지 못했습니다' />}
            {!isLoading && !isError && (data?.length ?? 0) === 0 && <EmptyState title='최근 제출이 없습니다' />}
            {!isLoading && !isError && (data?.length ?? 0) > 0 && (
                <DataTable columns={COLUMNS} rows={data ?? []} rowKey={(row) => row.score_id} caption='최근 제출 목록' />
            )}
            <PanelFootnote>최근 활동에는 ranked 제출만 표시됩니다.</PanelFootnote>
        </PanelCard>
    )
}
