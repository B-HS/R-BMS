import { buildPageMetadata } from '@shared/lib/metadata'
import { AiLimitsPanel } from '@widgets/admin/ai-limits-panel'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: 'AI 한도', description: 'AI 호출의 사용자별 한도.', index: false })

const AdminAiPage = () => (
    <ShellPage title='AI 한도' description='사용자별 AI 호출 한도'>
        <AiLimitsPanel />
    </ShellPage>
)

export default AdminAiPage
