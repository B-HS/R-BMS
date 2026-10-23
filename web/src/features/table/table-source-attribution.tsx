import type { FC } from 'react'
import { TableExternalLink } from '@features/table/table-external-link'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { formatDateTime } from '@shared/lib/format'

type TableSourceAttributionProps = {
    source: NonNullable<TableSnapshotSummary['source']>
}

const DATA_HASH_PREVIEW_LENGTH = 12

export const TableSourceAttribution: FC<TableSourceAttributionProps> = ({ source }) => (
    <section aria-label='표 출처' className='bg-card flex flex-col gap-1 px-3 py-2 text-xs leading-5 break-keep'>
        <p>
            출처 <span className='font-medium'>{source.name}</span>({source.maker}) ·{' '}
            <TableExternalLink href={source.page_url} label='원본 표' appearance='text' />
        </p>
        <p className='text-muted-foreground tabular-nums'>
            내용 받은 날 {formatDateTime(source.fetched_at)} · 마지막 확인 {formatDateTime(source.checked_at)} · SHA-256{' '}
            <code className='font-mono'>{source.data_sha256.slice(0, DATA_HASH_PREVIEW_LENGTH)}</code>
        </p>
    </section>
)
