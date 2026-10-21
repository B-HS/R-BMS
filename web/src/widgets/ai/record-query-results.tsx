import type { FC } from 'react'
import Link from 'next/link'
import { LampLabel } from '@features/lamp/lamp-label'
import { ScratchAutoBadge } from '@features/score/scratch-auto-badge'
import { EmptyState } from '@features/state/empty-state'
import { DataTable, type DataTableColumn } from '@features/table/data-table'
import { formatCount, formatDateTime } from '@shared/lib/format'
import type { AiRecordQueryAnswer, AiRecordQueryRow } from '@entities/ai.type'

const EX_RATE_DIGITS = 2
const MISSING = '—'

const SOURCE_NOTE = {
    scores: '내가 친 모든 판에서 찾았습니다.',
    table: '난이도표의 곡마다 가장 좋은 기록에서 찾았습니다. 아직 치지 않은 곡은 NO PLAY 로 보입니다.',
} satisfies Record<NonNullable<AiRecordQueryAnswer['result']>['source'], string>

const COLUMNS: DataTableColumn<AiRecordQueryRow>[] = [
    {
        key: 'title',
        label: '곡',
        flex: true,
        cell: (row) => (
            <span className='flex min-w-0 flex-col'>
                {row.chart_sha256 ? (
                    <Link href={`/charts/${row.chart_sha256}`} className='truncate hover:underline'>
                        {row.title || '제목 미등록'}
                    </Link>
                ) : (
                    <span className='truncate'>{row.title || '제목 미등록'}</span>
                )}
                {row.artist && <span className='text-muted-foreground truncate'>{row.artist}</span>}
            </span>
        ),
    },
    { key: 'level', label: '레벨', width: 64, cell: (row) => row.table_level ?? MISSING },
    { key: 'lamp', label: '램프', width: 144, cell: (row) => <LampLabel lamp={row.lamp} assistGrade={row.assist_grade} /> },
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
        label: '옵션',
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
        label: '친 날짜',
        width: 128,
        hideBelow: 'sm',
        mono: true,
        cell: (row) => (row.played_at === null ? MISSING : formatDateTime(row.played_at)),
    },
]

type RecordQueryResultsProps = {
    result: NonNullable<AiRecordQueryAnswer['result']>
}

export const RecordQueryResults: FC<RecordQueryResultsProps> = ({ result }) => (
    <section aria-label='질문 결과' className='flex flex-col gap-2'>
        <p className='text-muted-foreground text-xs'>
            {SOURCE_NOTE[result.source]} {formatCount(result.rows.length)}건.
        </p>
        {result.rows.length === 0 ? (
            <EmptyState title='조건에 맞는 기록이 없습니다' description='조건을 고쳐 다시 실행해 보세요.' />
        ) : (
            <DataTable
                caption='질문 결과'
                columns={COLUMNS}
                rows={result.rows}
                rowKey={(row) => row.score_id ?? `unplayed-${row.chart_md5 ?? row.title}`}
                rowMuted={(row) => row.score_id === null}
            />
        )}
    </section>
)
