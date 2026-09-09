'use client'

import type { FC } from 'react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@shared/ui/tabs'
import { PanelCard } from '@features/shell/panel-card'
import { ProfilePanel } from '@widgets/settings/profile-panel'
import { TokenPanel } from '@widgets/settings/token-panel'
import { SyncBlobPanel } from '@widgets/settings/sync-blob-panel'
import { RivalPanel } from '@widgets/settings/rival-panel'
import type { SessionUser } from '@entities/auth.type'

const TAB_CONTENT_CLASS = 'flex flex-col gap-px'

export const SettingsTabs: FC<{ user: SessionUser }> = ({ user }) => (
    <Tabs defaultValue='profile' className='flex flex-col gap-px'>
        <PanelCard title='계정 설정'>
            <TabsList>
                <TabsTrigger value='profile'>프로필</TabsTrigger>
                <TabsTrigger value='tokens'>API 토큰</TabsTrigger>
                <TabsTrigger value='sync'>동기화 blob</TabsTrigger>
                <TabsTrigger value='rivals'>라이벌</TabsTrigger>
            </TabsList>
        </PanelCard>
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
    </Tabs>
)
