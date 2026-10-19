import { AiPrincipleNote } from '@features/ai/ai-principle-note'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AiActiveProviderPanel } from '@widgets/ai/ai-active-provider-panel'
import { AiReadyGate } from '@widgets/ai/ai-ready-gate'
import { RecordQueryPanel } from '@widgets/ai/record-query-panel'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({ title: 'AI', description: '내 AI 키로 기록을 해석하는 기능 모음.', index: false })

const AiPage = () => (
    <ShellPage title='AI' description='내 키로 기록을 해석합니다'>
        <AiPrincipleNote />
        <AiReadyGate>
            <AiActiveProviderPanel />
            <RecordQueryPanel />
        </AiReadyGate>
    </ShellPage>
)

export default AiPage
