import { redirect } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { SettingsTabs } from '@widgets/settings/settings-tabs'
import { readSessionUser } from '@widgets/auth/session-gate'
import { apiTokenListQueryOptions } from '@entities/token.query'
import { settingKeyListQueryOptions } from '@entities/setting.query'
import { rivalListQueryOptions } from '@entities/rival.query'
import { buildLoginHref } from '@shared/constants/auth-route'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readApiTokens, readPlayerRivals, readSettingKeys } from '@server/service/public-reads'

const SETTINGS_PATH = '/settings'

export const SettingsContent = async () => {
    const user = await readSessionUser()
    if (!user) redirect(buildLoginHref(SETTINGS_PATH))

    const queryClient = getQueryClient()
    await Promise.all([
        queryClient.prefetchQuery({ ...apiTokenListQueryOptions(), queryFn: () => readApiTokens(user.id) }),
        queryClient.prefetchQuery({ ...settingKeyListQueryOptions(), queryFn: () => readSettingKeys(user.id) }),
        queryClient.prefetchQuery({ ...rivalListQueryOptions(user.login_id), queryFn: () => readPlayerRivals(user.login_id) }),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <SettingsTabs user={user} />
        </HydrationBoundary>
    )
}
