import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { TableExternalLink } from '@features/table/table-external-link'
import type { TableSnapshotSummary } from '@entities/table-snapshot.type'
import { formatDateTime } from '@shared/lib/format'

type TableSourceAttributionProps = {
    source: NonNullable<TableSnapshotSummary['source']>
}

const DATA_HASH_PREVIEW_LENGTH = 12

export const TableSourceAttribution: FC<TableSourceAttributionProps> = ({ source }) => {
    const t = useTranslations('pages.table.source')

    return (
        <section aria-label={t('ariaLabel')} className='bg-card flex flex-col gap-1 px-3 py-2 text-xs leading-5 break-keep'>
            <p>
                {t('label')} <span className='font-medium'>{source.name}</span>({source.maker}) ·{' '}
                <TableExternalLink href={source.page_url} label={t('originalTable')} appearance='text' />
            </p>
            <p className='text-muted-foreground tabular-nums'>
                {t('fetchedAt', { date: formatDateTime(source.fetched_at) })} · {t('checkedAt', { date: formatDateTime(source.checked_at) })} ·
                SHA-256 <code className='font-mono'>{source.data_sha256.slice(0, DATA_HASH_PREVIEW_LENGTH)}</code>
            </p>
        </section>
    )
}
