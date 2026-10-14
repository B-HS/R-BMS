import { Suspense } from 'react'
import { RowListSkeleton } from '@features/state/row-list-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPage } from '@widgets/shell/shell-page'
import { TableListContent } from '@widgets/table/table-list-content'

export const metadata = buildPageMetadata({ title: '난이도표', description: 'R-BMS 점수 서버가 제공하는 BMS 난이도표 목록.', path: '/tables' })

const TablesPage = () => (
    <ShellPage title='난이도표' description='서버가 제공하는 표'>
        <Suspense fallback={<RowListSkeleton label='난이도표 목록 불러오는 중' />}>
            <TableListContent />
        </Suspense>
    </ShellPage>
)

export default TablesPage
