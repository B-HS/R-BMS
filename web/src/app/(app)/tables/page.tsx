import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPage } from '@widgets/shell/shell-page'
import { TableCatalogContent } from '@widgets/table/table-catalog-content'

export const metadata = buildPageMetadata({
    title: '난이도표',
    description: 'R-BMS 에 기본 등록된 BMS 난이도표와 점수 서버가 제공하는 표.',
    path: '/tables',
})

const TablesPage = () => (
    <ShellPage title='난이도표' description='R-BMS 에 기본 등록된 표와 서버가 제공하는 표'>
        <TableCatalogContent />
    </ShellPage>
)

export default TablesPage
