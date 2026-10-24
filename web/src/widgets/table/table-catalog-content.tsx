import 'server-only'
import { Suspense } from 'react'
import { connection } from 'next/server'
import { getTranslations } from 'next-intl/server'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { TableListContent } from '@widgets/table/table-list-content'
import { TableSnapshotListItem } from '@widgets/table/table-snapshot-list-item'
import { SITE_URL } from '@shared/constants/site'
import { compose } from '@server/compose'

export const TableCatalogContent = async () => {
    await connection()
    const t = await getTranslations('browse.table.catalog')
    const defaultTables = await compose().tableSnapshotService.list()

    return (
        <>
            <p className='text-muted-foreground bg-card px-3 py-2 text-xs leading-5 break-keep'>{t('intro')}</p>
            <section aria-labelledby='bundled-tables-heading' className='bg-card flex flex-col'>
                <h2 id='bundled-tables-heading' className='rbms-micro-label flex h-8 items-center px-3'>
                    {t('bundledHeading')}
                </h2>
                <ul className='bg-border flex flex-col gap-px border-t'>
                    {defaultTables.map((table) => (
                        <TableSnapshotListItem key={table.id} table={table} tableUrl={`${SITE_URL}${table.standard_paths.page}`} />
                    ))}
                </ul>
            </section>
            <section aria-labelledby='server-tables-heading' className='bg-card flex flex-col'>
                <h2 id='server-tables-heading' className='rbms-micro-label flex h-8 items-center px-3'>
                    {t('serverHeading')}
                </h2>
                <Suspense fallback={<RowListSkeleton rows={1} label={t('serverLoading')} />}>
                    <TableListContent />
                </Suspense>
            </section>
        </>
    )
}
