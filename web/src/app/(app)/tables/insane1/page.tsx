import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { TableSnapshotContent } from '@widgets/table/table-snapshot-content'

const TABLE_ID = 'insane1'
const TABLE_PATH = `/tables/${TABLE_ID}`

export const generateMetadata = async (): Promise<Metadata> => {
    const t = await getTranslations('pages.route.insane1')
    return {
        ...(await buildPageMetadata({ title: '発狂BMS難易度表', description: t('metaDescription'), path: TABLE_PATH, index: false })),
        other: { bmstable: `${TABLE_PATH}/header.json` },
    }
}

const InsaneTablePage = () => <TableSnapshotContent tableId={TABLE_ID} />

export default InsaneTablePage
