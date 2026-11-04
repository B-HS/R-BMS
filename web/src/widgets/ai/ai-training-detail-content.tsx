import { notFound } from 'next/navigation'
import { getTranslations } from 'next-intl/server'
import { AiPrincipleNote } from '@features/ai/ai-principle-note'
import { AiTrainingDetailPanel } from '@widgets/ai/ai-training-detail-panel'
import { ShellPage } from '@widgets/shell/shell-page'
import { isAiTrainingChartId } from '@entities/ai.type'

/** `/ai/training/[id]`: one own training chart. A path that is not a chart id is a missing page without asking the server; the panel reads the session user's chart only. */
export const AiTrainingDetailContent = async ({ params }: { params: Promise<{ id: string }> }) => {
    const { id } = await params
    if (!isAiTrainingChartId(id)) notFound()

    const t = await getTranslations('ai.trainingDetail')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <AiPrincipleNote />
            <AiTrainingDetailPanel chartId={id} />
        </ShellPage>
    )
}
