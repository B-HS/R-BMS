import type { Metadata } from 'next'
import { buildPageMetadata } from '@shared/lib/metadata'
import { TableSnapshotContent } from '@widgets/table/table-snapshot-content'

const TABLE_ID = 'insane1'
const TABLE_PATH = `/tables/${TABLE_ID}`

export const metadata = {
    ...buildPageMetadata({
        title: '発狂BMS難易度表',
        description: '発狂BMS難易度表(★1 ~ ★25)의 보관본 사본. R-BMS 에 표 주소로 등록하고 레벨별로 내 램프를 봅니다.',
        path: TABLE_PATH,
        index: false,
    }),
    other: { bmstable: `${TABLE_PATH}/header.json` },
} satisfies Metadata

const InsaneTablePage = () => <TableSnapshotContent tableId={TABLE_ID} />

export default InsaneTablePage
