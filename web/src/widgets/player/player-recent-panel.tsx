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
import { usePlayerRecent } from '@entities/player.query'
import type { PlayerRecentRow } from '@entities/player.type'
import { formatCount, formatDateTime } from '@shared/lib/format'

const RECENT_LIMIT = 20

const COLUMNS: DataTableColumn<PlayerRecentRow>[] = [
    { key: 'played_at', label: '시각', width: 160, mono: true, cell: (row) => formatDateTime(row.played_at) },
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
    { key: 'max_combo', label: '콤보', width: 96, align: 'right', cell: (row) => formatCount(row.max_combo) },
    { key: 'minbp', label: 'BP', width: 64, align: 'right', cell: (row) => formatCount(row.minbp) },
]

export const PlayerRecentPanel: FC<{ playerId: string }> = ({ playerId }) => {
    const { data, isLoading, isError } = usePlayerRecent(playerId, { limit: RECENT_LIMIT })

    return (
        <PanelCard title='최근 기록' contentClassName='flex flex-col gap-2'>
            {isLoading && <TableSkeleton />}
            {!isLoading && isError && <LoadErrorState title='최근 기록을 불러오지 못했습니다' />}
            {!isLoading && !isError && (data?.length ?? 0) === 0 && <EmptyState title='기록이 없습니다' />}
            {!isLoading && !isError && (data?.length ?? 0) > 0 && (
                <DataTable columns={COLUMNS} rows={data ?? []} rowKey={(row) => row.score_id} caption='플레이어 최근 기록' />
            )}
            <PanelFootnote>ranked 여부와 무관하게 최근 제출 순으로 표시됩니다.</PanelFootnote>
        </PanelCard>
    )
}
