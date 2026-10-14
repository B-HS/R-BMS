import { redirect } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { SettingsTabs } from '@widgets/settings/settings-tabs'
import { readSessionUser } from '@widgets/auth/session-gate'
import { apiTokenListQueryOptions } from '@entities/token.query'
import { settingKeyListQueryOptions } from '@entities/setting.query'
import { rivalListQueryOptions } from '@entities/rival.query'
import { buildLoginHref } from '@shared/constants/auth-route'
import { getQueryClient } from '@shared/utils/get-query-client'
import { forwardedCookieInit } from '@shared/lib/server-request-init'

const SETTINGS_PATH = '/settings'

export const SettingsContent = async () => {
    const user = await readSessionUser()
    if (!user) redirect(buildLoginHref(SETTINGS_PATH))

    const queryClient = getQueryClient()
    const init = await forwardedCookieInit()
    await Promise.all([
        queryClient.prefetchQuery(apiTokenListQueryOptions(init)),
        queryClient.prefetchQuery(settingKeyListQueryOptions(init)),
        queryClient.prefetchQuery(rivalListQueryOptions(user.login_id, init)),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <SettingsTabs user={user} />
        </HydrationBoundary>
    )
}
