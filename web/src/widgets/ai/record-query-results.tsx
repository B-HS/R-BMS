'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { LampLabel } from '@features/lamp/lamp-label'
import { ScratchAutoBadge } from '@features/score/scratch-auto-badge'
import { EmptyState } from '@features/state/empty-state'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { formatCount, formatDateTime } from '@shared/lib/format'
import type { AiRecordQueryAnswer, AiRecordQueryRow } from '@entities/ai.type'

const EX_RATE_DIGITS = 2
const MISSING = '—'

const SOURCE_SUMMARY_KEY = {
    scores: 'summaryScores',
    table: 'summaryTable',
} as const satisfies Record<NonNullable<AiRecordQueryAnswer['result']>['source'], string>

type RecordQueryResultsProps = {
    result: NonNullable<AiRecordQueryAnswer['result']>
}

export const RecordQueryResults: FC<RecordQueryResultsProps> = ({ result }) => {
    const t = useTranslations('ai.recordQueryResults')

    const columns: DataTableColumn<AiRecordQueryRow>[] = [
        {
            key: 'title',
            label: t('columnTitle'),
            flex: true,
            cell: (row) => (
                <span className='flex min-w-0 flex-col'>
                    {row.chart_sha256 ? (
                        <Link href={`/charts/${row.chart_sha256}`} className='truncate hover:underline'>
                            {row.title || t('untitled')}
                        </Link>
                    ) : (
                        <span className='truncate'>{row.title || t('untitled')}</span>
                    )}
                    {row.artist && <span className='text-muted-foreground truncate'>{row.artist}</span>}
                </span>
            ),
        },
        { key: 'level', label: t('columnLevel'), width: 64, cell: (row) => row.table_level ?? MISSING },
        { key: 'lamp', label: t('columnLamp'), width: 144, cell: (row) => <LampLabel lamp={row.lamp} assistGrade={row.assist_grade} /> },
        {
            key: 'ex',
            label: 'EX',
            width: 96,
            align: 'right',
            cell: (row) => (row.ex_rate === null ? MISSING : `${row.ex_rate.toFixed(EX_RATE_DIGITS)}%`),
        },
        { key: 'bp', label: 'BP', width: 56, align: 'right', cell: (row) => (row.bp === null ? MISSING : formatCount(row.bp)) },
        {
            key: 'option',
            label: t('columnOption'),
            width: 136,
            hideBelow: 'md',
            cell: (row) => (
                <span className='inline-flex items-center gap-1'>
                    {row.random ?? MISSING}
                    {row.gauge && <span className='text-muted-foreground'>· {row.gauge}</span>}
                    {row.scratch_auto && <ScratchAutoBadge />}
                </span>
            ),
        },
        {
            key: 'played_at',
            label: t('columnPlayedAt'),
            width: 128,
            hideBelow: 'sm',
            mono: true,
            cell: (row) => (row.played_at === null ? MISSING : formatDateTime(row.played_at)),
        },
    ]

    return (
        <section aria-label={t('regionLabel')} className='flex flex-col gap-2'>
            <p className='text-muted-foreground text-xs'>{t(SOURCE_SUMMARY_KEY[result.source], { count: result.rows.length })}</p>
            {result.rows.length === 0 ? (
                <EmptyState title={t('emptyTitle')} description={t('emptyDescription')} />
            ) : (
                <DataTable
                    caption={t('caption')}
                    columns={columns}
                    rows={result.rows}
                    rowKey={(row) => row.score_id ?? `unplayed-${row.chart_md5 ?? row.title}`}
                    rowMuted={(row) => row.score_id === null}
                />
            )}
        </section>
    )
}
