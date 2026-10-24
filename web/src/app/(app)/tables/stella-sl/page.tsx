import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { TableSnapshotContent } from '@widgets/table/table-snapshot-content'

const TABLE_ID = 'stella-sl'
const TABLE_TITLE = 'Satellite'
const TABLE_PATH = `/tables/${TABLE_ID}`

export const generateMetadata = async (): Promise<Metadata> => {
    const t = await getTranslations('pages.route.tableSnapshot')
    return {
        ...(await buildPageMetadata({
            title: TABLE_TITLE,
            description: t('metaDescription', { name: TABLE_TITLE }),
            path: TABLE_PATH,
            index: false,
        })),
        other: { bmstable: `${TABLE_PATH}/header.json` },
    }
}

const SatelliteTablePage = () => (
    <Suspense fallback={<ShellPageLoading title={TABLE_TITLE} />}>
        <TableSnapshotContent tableId={TABLE_ID} />
    </Suspense>
)

export default SatelliteTablePage
