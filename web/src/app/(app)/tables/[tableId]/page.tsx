import { Suspense } from 'react'
import type { Metadata } from 'next'
import { tableDetailQueryOptions } from '@entities/table.query'
import { buildPageMetadata } from '@shared/lib/metadata'
import { getQueryClient } from '@shared/utils/get-query-client'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { TableDetailContent } from '@widgets/table/table-detail-content'

export const generateMetadata = async ({ params }: PageProps<'/tables/[tableId]'>): Promise<Metadata> => {
    const { tableId } = await params
    const table = await getQueryClient()
        .fetchQuery(tableDetailQueryOptions(tableId))
        .catch(() => null)
    if (!table) return buildPageMetadata({ title: '난이도표', index: false })
    return buildPageMetadata({
        title: table.name,
        description: `${table.name} 난이도표의 차트 목록.`,
        path: `/tables/${encodeURIComponent(table.id)}`,
    })
}

const TableDetailPage = ({ params }: PageProps<'/tables/[tableId]'>) => (
    <Suspense fallback={<ShellPageLoading title='난이도표' />}>
        <TableDetailContent params={params} />
    </Suspense>
)

export default TableDetailPage
