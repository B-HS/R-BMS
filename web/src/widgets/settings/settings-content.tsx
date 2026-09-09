import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { SettingsTabs } from '@widgets/settings/settings-tabs'
import { readSessionUser } from '@widgets/auth/session-gate'
import { apiTokenListQueryOptions } from '@entities/token.query'
import { settingKeyListQueryOptions } from '@entities/setting.query'
import { rivalListQueryOptions } from '@entities/rival.query'
import { getQueryClient } from '@shared/utils/get-query-client'
import { forwardedCookieInit } from '@shared/lib/server-request-init'
import { PageHeading } from '@features/shell/page-heading'
import { EmptyState } from '@features/state/empty-state'

export const SettingsContent = async () => {
    const user = await readSessionUser()
    if (!user) return <EmptyState title='세션이 만료되었습니다' description='다시 로그인하세요.' />

    const queryClient = getQueryClient()
    const init = await forwardedCookieInit()
    await Promise.all([
        queryClient.prefetchQuery(apiTokenListQueryOptions(init)),
        queryClient.prefetchQuery(settingKeyListQueryOptions(init)),
        queryClient.prefetchQuery(rivalListQueryOptions(user.login_id, init)),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <PageHeading>계정 설정</PageHeading>
            <SettingsTabs user={user} />
        </HydrationBoundary>
    )
}
