'use client'

import { useState, type FC } from 'react'
import { useSearchParams } from 'next/navigation'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@shared/ui/tabs'
import { AI_HUB_PARAM, AI_HUB_TABS, AI_HUB_TAB_LABEL, type AiHubTab } from '@entities/ai.type'
import { AiActiveProviderPanel } from '@widgets/ai/ai-active-provider-panel'
import { AiGaugeAdvicePanel } from '@widgets/ai/ai-gauge-advice-panel'
import { AiGoalPanel } from '@widgets/ai/ai-goal-panel'
import { AiJudgeHabitPanel } from '@widgets/ai/ai-judge-habit-panel'
import { AiPracticeFolderPanel } from '@widgets/ai/ai-practice-folder-panel'
import { AiProfilePanel } from '@widgets/ai/ai-profile-panel'
import { AiReadyGate } from '@widgets/ai/ai-ready-gate'
import { AiRecommendPanel } from '@widgets/ai/ai-recommend-panel'
import { AiReportPanel } from '@widgets/ai/ai-report-panel'
import { AiWeeklyPanel } from '@widgets/ai/ai-weekly-panel'
import { RecordQueryPanel } from '@widgets/ai/record-query-panel'

const TAB_CONTENT_CLASS = 'flex flex-col gap-px'
const DEFAULT_TAB = 'query' satisfies AiHubTab
const PANEL_PENDING_TABS: readonly AiHubTab[] = ['training']
const SHOWN_TABS = AI_HUB_TABS.filter((tab) => !PANEL_PENDING_TABS.includes(tab))

const shownTabOf = (requested: string | null) => SHOWN_TABS.find((tab) => tab === requested) ?? DEFAULT_TAB

/**
 * The AI hub. The record question needs a key before it can do anything, so it sits behind the ready gate; the other features
 * show their numbers without a key and ask for one only when the user presses the AI button. A link with `tab` (and `focus`, a
 * weakness key) opens that tab on that weakness; switching tabs writes the tab back into the address without a navigation.
 */
export const AiHubTabs: FC = () => {
    const searchParams = useSearchParams()
    const requestedTab = searchParams.get(AI_HUB_PARAM.TAB)
    const focusKey = searchParams.get(AI_HUB_PARAM.FOCUS)
    const request = `${requestedTab ?? ''}|${focusKey ?? ''}`
    const [tab, setTab] = useState<AiHubTab>(() => shownTabOf(requestedTab))
    const [seenRequest, setSeenRequest] = useState(request)

    if (request !== seenRequest) {
        setSeenRequest(request)
        setTab(shownTabOf(requestedTab))
    }

    const handleTabChange = (next: string) => {
        const nextTab = shownTabOf(next)
        setTab(nextTab)
        const params = new URLSearchParams({ [AI_HUB_PARAM.TAB]: nextTab })
        window.history.replaceState(null, '', `?${params.toString()}`)
    }

    return (
        <Tabs value={tab} onValueChange={handleTabChange} className='flex flex-col gap-px'>
            <div className='border-border bg-card overflow-x-auto border-b px-3 py-2'>
                <TabsList>
                    {SHOWN_TABS.map((value) => (
                        <TabsTrigger key={value} value={value}>
                            {AI_HUB_TAB_LABEL[value]}
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
            <TabsContent value='profile' className={TAB_CONTENT_CLASS}>
                <AiProfilePanel />
            </TabsContent>
            <TabsContent value='recommend' className={TAB_CONTENT_CLASS}>
                <AiRecommendPanel />
                <AiPracticeFolderPanel key={focusKey ?? ''} initialFocusKey={focusKey} />
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
