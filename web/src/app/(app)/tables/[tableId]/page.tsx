import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { readTable } from '@server/service/public-reads'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { TableDetailContent } from '@widgets/table/table-detail-content'

export const generateMetadata = async ({ params }: PageProps<'/tables/[tableId]'>): Promise<Metadata> => {
    const { tableId } = await params
    const [t, table] = await Promise.all([getTranslations('pages.route.tableDetail'), readTable(tableId).catch(() => null)])
    if (!table) return buildPageMetadata({ title: t('title'), index: false })
    return buildPageMetadata({
        title: table.name,
        description: t('metaDescription', { name: table.name }),
        path: `/tables/${encodeURIComponent(table.id)}`,
    })
}

const TableDetailPage = async ({ params }: PageProps<'/tables/[tableId]'>) => {
    const t = await getTranslations('pages.route.tableDetail')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <TableDetailContent params={params} />
        </Suspense>
    )
}

export default TableDetailPage
