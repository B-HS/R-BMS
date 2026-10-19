import { notFound } from 'next/navigation'
import { AiPrincipleNote } from '@features/ai/ai-principle-note'
import { AiCoachPanel } from '@widgets/ai/ai-coach-panel'
import { AiComparePanel } from '@widgets/ai/ai-compare-panel'
import { ShellPage } from '@widgets/shell/shell-page'
import { isAiScoreId } from '@entities/ai.type'

/** `/ai/plays/[scoreId]`: the coach and the comparison of one own play. The panels read the session user's data only. */
export const AiPlayDetailContent = async ({ params }: { params: Promise<{ scoreId: string }> }) => {
    const { scoreId } = await params
    if (!isAiScoreId(scoreId)) notFound()

    return (
        <ShellPage title='판 코치' description='내 판 하나를 구간별로 봅니다'>
            <AiPrincipleNote />
            <AiCoachPanel scoreId={scoreId} />
            <AiComparePanel scoreId={scoreId} />
        </ShellPage>
    )
}
