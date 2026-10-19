import { AiPrincipleNote } from '@features/ai/ai-principle-note'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiHubTabs } from '@widgets/ai/ai-hub-tabs'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: 'AI', description: '내 AI 키로 기록을 해석하는 기능 모음.', index: false })

const AiPage = () => (
    <ShellPage title='AI' description='내 키로 기록을 해석합니다'>
        <AiPrincipleNote />
        <AiHubTabs />
    </ShellPage>
)

export default AiPage
