import { notFound } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { AiPrincipleNote } from '@features/ai/ai-principle-note'
import { AiCoachPanel } from '@widgets/ai/ai-coach-panel'
import { AiComparePanel } from '@widgets/ai/ai-compare-panel'
import { ShellPage } from '@widgets/shell/shell-page'
import { isAiScoreId } from '@entities/ai.type'

/** `/ai/plays/[scoreId]`: the coach and the comparison of one own play. The panels read the session user's data only. */
export const AiPlayDetailContent = async ({ params }: { params: Promise<{ scoreId: string }> }) => {
    const { scoreId } = await params
    if (!isAiScoreId(scoreId)) notFound()

    const t = await getTranslations('ai.playDetail')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <AiPrincipleNote />
            <AiCoachPanel scoreId={scoreId} />
            <AiComparePanel scoreId={scoreId} />
        </ShellPage>
    )
}
