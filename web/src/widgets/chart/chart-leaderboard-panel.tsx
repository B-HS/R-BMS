'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { PanelCard } from '@features/shell/panel-card'
import { PanelFootnote } from '@features/shell/panel-footnote'
import { StatTile } from '@features/shell/stat-tile'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { LampBadge } from '@features/badge/lamp-badge'
import { MetaBadge } from '@features/badge/meta-badge'
import { EmptyState } from '@features/state/empty-state'
import { LoadErrorState } from '@features/state/load-error-state'
import { TableSkeleton } from '@features/state/table-skeleton'
import { useChartLeaderboard } from '@entities/chart.query'
import type { ScoreRecord } from '@entities/score.type'
import { CLEAR_LAMPS, CLEAR_LAMP_LABEL } from '@shared/constants/clear-lamp'
import { formatCount, formatDateTime, formatHash } from '@shared/lib/format'

const RANKING_SCOPE_HINT = '서버가 내려주는 상위 구간 기준'

const formatBpmRange = (minbpm: number, maxbpm: number) => (minbpm === maxbpm ? String(minbpm) : `${minbpm}–${maxbpm}`)

const LN_TYPE_LABEL: Record<number, string> = { 0: 'LN', 1: 'CN', 2: 'HCN' }

const COLUMNS: DataTableColumn<ScoreRecord>[] = [
    { key: 'rank', label: '순위', width: 64, align: 'right', cell: (row) => row.rank ?? '—' },
    {
        key: 'player',
        label: '플레이어',
        flex: true,
        cell: (row) => (
            <Link href={`/players/${row.player.id}`} className='hover:underline'>
                {row.player_name}
            </Link>
        ),
    },
    { key: 'clear', label: '램프', width: 128, cell: (row) => <LampBadge clear={row.clear} /> },
    { key: 'ex_score', label: 'EX', width: 96, align: 'right', cell: (row) => formatCount(row.ex_score) },
    { key: 'max_combo', label: '콤보', width: 96, align: 'right', cell: (row) => formatCount(row.max_combo) },
    { key: 'minbp', label: 'BP', width: 64, align: 'right', cell: (row) => formatCount(row.minbp) },
    { key: 'played_at', label: '기록 시각', width: 160, mono: true, cell: (row) => formatDateTime(row.played_at) },
]

export const ChartLeaderboardPanel: FC<{ hash: string }> = ({ hash }) => {
    const { data, isLoading, isError } = useChartLeaderboard(hash)

    if (isLoading)
        return (
            <PanelCard title='리더보드'>
                <TableSkeleton rows={8} />
            </PanelCard>
        )

    if (isError) return <LoadErrorState title='리더보드를 불러오지 못했습니다' />

    if (!data?.chart) return <EmptyState title='등록되지 않은 차트입니다' description='이 해시로 제출된 기록이 아직 없습니다.' />

    const { chart, ranking } = data
    const bestExScore = ranking.reduce((best, row) => Math.max(best, row.ex_score), 0)
    const bestClear = ranking.reduce<ScoreRecord['clear']>(
        (best, row) => (CLEAR_LAMPS.indexOf(row.clear) > CLEAR_LAMPS.indexOf(best) ? row.clear : best),
        'NoPlay',
    )

    return (
        <>
            <PanelCard contentClassName='flex flex-col gap-2'>
                <h1 className='text-xl font-semibold tracking-tight'>{chart.title}</h1>
                <p className='text-muted-foreground text-sm'>
                    {chart.subtitle} · {chart.artist}
                </p>
                <div className='flex flex-wrap items-center gap-1'>
                    <MetaBadge>{chart.mode}</MetaBadge>
                    <MetaBadge>Lv {chart.level ?? '—'}</MetaBadge>
                    <MetaBadge>{LN_TYPE_LABEL[chart.lntype] ?? 'LN'}</MetaBadge>
                    <MetaBadge>{formatCount(chart.notes)} notes</MetaBadge>
                </div>
                <p className='text-muted-foreground flex flex-wrap gap-3 text-xs'>
                    <span>BPM {formatBpmRange(chart.minbpm, chart.maxbpm)}</span>
                    <span>TOTAL {chart.total ?? '—'}</span>
                    <span className='font-mono'>md5 {formatHash(chart.md5)}</span>
                    <span className='font-mono'>sha256 {formatHash(chart.sha256)}</span>
                </p>
            </PanelCard>

            <div className='grid grid-cols-2 gap-px md:grid-cols-4'>
                <StatTile label='최고 EX' value={formatCount(bestExScore)} />
                <StatTile label='최고 램프' value={CLEAR_LAMP_LABEL[bestClear]} />
                <StatTile label='랭킹 내 플레이어' value={formatCount(new Set(ranking.map((row) => row.player.id)).size)} hint={RANKING_SCOPE_HINT} />
                <StatTile label='랭킹 표시' value={formatCount(ranking.length)} hint={RANKING_SCOPE_HINT} />
            </div>

            <PanelCard title='랭킹' contentClassName='flex flex-col gap-2'>
                {ranking.length === 0 && <EmptyState title='등록된 기록이 없습니다' />}
                {ranking.length > 0 && (
                    <DataTable columns={COLUMNS} rows={ranking} rowKey={(row) => `${row.player.id}-${row.played_at}`} caption='차트 랭킹' />
                )}
                <PanelFootnote>unranked 기록은 랭킹에 표시되지 않습니다.</PanelFootnote>
            </PanelCard>
        </>
    )
}
