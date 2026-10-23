import { Suspense } from 'react'
import type { Metadata } from 'next'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'
import { TableSnapshotContent } from '@widgets/table/table-snapshot-content'

const TABLE_ID = 'stella-sl'
const TABLE_TITLE = 'Satellite'
const TABLE_PATH = `/tables/${TABLE_ID}`

export const metadata = {
    ...buildPageMetadata({
        title: TABLE_TITLE,
        description: 'Satellite 난이도표. R-BMS 에 표 주소로 등록하고 레벨별로 내 램프를 봅니다.',
        path: TABLE_PATH,
        index: false,
    }),
    other: { bmstable: `${TABLE_PATH}/header.json` },
} satisfies Metadata

const SatelliteTablePage = () => (
    <Suspense fallback={<ShellPageLoading title={TABLE_TITLE} />}>
        <TableSnapshotContent tableId={TABLE_ID} />
    </Suspense>
)

export default SatelliteTablePage
