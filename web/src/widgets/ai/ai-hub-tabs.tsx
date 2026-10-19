'use client'

import type { FC } from 'react'
import { useSearchParams } from 'next/navigation'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@shared/ui/tabs'
import { AI_HUB_TABS, AI_HUB_TAB_LABEL, type AiHubTab } from '@entities/ai.type'
import { AiActiveProviderPanel } from '@widgets/ai/ai-active-provider-panel'
import { AiGaugeAdvicePanel } from '@widgets/ai/ai-gauge-advice-panel'
import { AiGoalPanel } from '@widgets/ai/ai-goal-panel'
import { AiJudgeHabitPanel } from '@widgets/ai/ai-judge-habit-panel'
import { AiPracticeFolderPanel } from '@widgets/ai/ai-practice-folder-panel'
import { AiReadyGate } from '@widgets/ai/ai-ready-gate'
import { AiRecommendPanel } from '@widgets/ai/ai-recommend-panel'
import { AiReportPanel } from '@widgets/ai/ai-report-panel'
import { AiWeeklyPanel } from '@widgets/ai/ai-weekly-panel'
import { RecordQueryPanel } from '@widgets/ai/record-query-panel'

const TAB_PARAM = 'tab'
const TAB_CONTENT_CLASS = 'flex flex-col gap-px'
const DEFAULT_TAB = 'query' satisfies AiHubTab

/**
 * The AI hub. The record question needs a key before it can do anything, so it sits behind the ready gate; the other features
 * show their numbers without a key and ask for one only when the user presses the AI button.
 */
export const AiHubTabs: FC = () => {
    const requestedTab = useSearchParams().get(TAB_PARAM)

    return (
        <Tabs defaultValue={AI_HUB_TABS.find((tab) => tab === requestedTab) ?? DEFAULT_TAB} className='flex flex-col gap-px'>
            <div className='border-border bg-card overflow-x-auto border-b px-3 py-2'>
                <TabsList>
                    {AI_HUB_TABS.map((tab) => (
                        <TabsTrigger key={tab} value={tab}>
                            {AI_HUB_TAB_LABEL[tab]}
                        </TabsTrigger>
                    ))}
                </TabsList>
            </div>
            <TabsContent value='query' className={TAB_CONTENT_CLASS}>
                <AiReadyGate>
                    <AiActiveProviderPanel />
                    <RecordQueryPanel />
                </AiReadyGate>
            </TabsContent>
            <TabsContent value='recommend' className={TAB_CONTENT_CLASS}>
                <AiRecommendPanel />
                <AiPracticeFolderPanel />
            </TabsContent>
            <TabsContent value='goals' className={TAB_CONTENT_CLASS}>
                <AiGoalPanel />
            </TabsContent>
            <TabsContent value='weekly' className={TAB_CONTENT_CLASS}>
                <AiWeeklyPanel />
            </TabsContent>
            <TabsContent value='report' className={TAB_CONTENT_CLASS}>
                <AiReportPanel />
            </TabsContent>
            <TabsContent value='habit' className={TAB_CONTENT_CLASS}>
                <AiJudgeHabitPanel />
            </TabsContent>
            <TabsContent value='gauge' className={TAB_CONTENT_CLASS}>
                <AiGaugeAdvicePanel />
            </TabsContent>
        </Tabs>
    )
}
