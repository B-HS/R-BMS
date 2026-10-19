'use client'

import type { FC } from 'react'
import { useSearchParams } from 'next/navigation'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@shared/ui/tabs'
import { AI_SETTINGS_TAB, SETTINGS_TAB_PARAM } from '@shared/constants/navigation'
import { AiCredentialPanel } from '@widgets/settings/ai-credential-panel'
import { AiUsagePanel } from '@widgets/settings/ai-usage-panel'
import { ProfilePanel } from '@widgets/settings/profile-panel'
import { TokenPanel } from '@widgets/settings/token-panel'
import { SyncBlobPanel } from '@widgets/settings/sync-blob-panel'
import { RivalPanel } from '@widgets/settings/rival-panel'
import type { SessionUser } from '@entities/auth.type'

const TAB_CONTENT_CLASS = 'flex flex-col gap-px'
const DEFAULT_TAB = 'profile'
const SETTINGS_TABS = [DEFAULT_TAB, 'tokens', 'sync', 'rivals', AI_SETTINGS_TAB]

export const SettingsTabs: FC<{ user: SessionUser }> = ({ user }) => {
    const requestedTab = useSearchParams().get(SETTINGS_TAB_PARAM)

    return (
        <Tabs defaultValue={SETTINGS_TABS.find((tab) => tab === requestedTab) ?? DEFAULT_TAB} className='flex flex-col gap-px'>
            <div className='border-border bg-card overflow-x-auto border-b px-3 py-2'>
                <TabsList>
                    <TabsTrigger value='profile'>프로필</TabsTrigger>
                    <TabsTrigger value='tokens'>API 토큰</TabsTrigger>
                    <TabsTrigger value='sync'>동기화 blob</TabsTrigger>
                    <TabsTrigger value='rivals'>라이벌</TabsTrigger>
                    <TabsTrigger value={AI_SETTINGS_TAB}>AI 키</TabsTrigger>
                </TabsList>
            </div>
            <TabsContent value='profile' className={TAB_CONTENT_CLASS}>
                <ProfilePanel user={user} />
            </TabsContent>
            <TabsContent value='tokens' className={TAB_CONTENT_CLASS}>
                <TokenPanel />
            </TabsContent>
            <TabsContent value='sync' className={TAB_CONTENT_CLASS}>
                <SyncBlobPanel playerId={user.login_id} />
            </TabsContent>
            <TabsContent value='rivals' className={TAB_CONTENT_CLASS}>
                <RivalPanel playerId={user.login_id} />
            </TabsContent>
            <TabsContent value={AI_SETTINGS_TAB} className={TAB_CONTENT_CLASS}>
                <AiCredentialPanel />
                <AiUsagePanel />
            </TabsContent>
        </Tabs>
    )
}
