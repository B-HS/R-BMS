import { buildPageMetadata } from '@shared/lib/metadata'
import { GuideArticle } from '@widgets/guide/guide-article'
import { GuideToc } from '@widgets/guide/guide-toc'
import { ShellPage } from '@widgets/shell/shell-page'
import { ShellSidebarPortal } from '@widgets/shell/shell-sidebar-portal'

export const metadata = buildPageMetadata({
    title: '가이드',
    description: 'R-BMS 플레이어를 점수 서버에 연결하는 방법: 서버 주소, 계정, 순위, 리플레이, 설정 동기화, 라이벌.',
    path: '/guide',
})

const GuidePage = () => (
    <ShellPage title='가이드' description='게임 안에서 점수 서버에 연결하기' contentClassName='gap-0'>
        <GuideArticle />
        <ShellSidebarPortal>
            <GuideToc />
        </ShellSidebarPortal>
    </ShellPage>
)

export default GuidePage
