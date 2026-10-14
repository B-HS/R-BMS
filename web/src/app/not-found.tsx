import { NotFoundNotice } from '@features/state/not-found-notice'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: '404', description: '주소가 바뀌었거나 없는 페이지입니다.', index: false })

const NotFoundPage = () => (
    <ShellPage title='찾을 수 없음'>
        <NotFoundNotice />
    </ShellPage>
)

export default NotFoundPage
