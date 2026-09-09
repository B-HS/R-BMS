import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchRaw } from '@shared/lib/fetch'
import type { SettingBlob, SettingBlobKey } from '@entities/setting.type'

export const settingKeyListQueryOptions = (init?: RequestInit) =>
    queryOptions({
        queryKey: QUERY_KEY.SETTING.LIST,
        queryFn: async () => (await fetchEnvelope<{ keys: SettingBlobKey[] }>(buildApiUrl('/fe/me/settings'), init)).keys,
    })

export const settingBlobQueryOptions = (playerId: string, key: string) =>
    queryOptions({
        queryKey: QUERY_KEY.SETTING.DETAIL(playerId, key),
        queryFn: () => fetchRaw<SettingBlob>(buildApiUrl(`/players/${encodeURIComponent(playerId)}/settings/${encodeURIComponent(key)}`)),
        enabled: Boolean(key),
    })

export const useSettingKeyList = () => useQuery(settingKeyListQueryOptions())

export const useSettingBlob = (playerId: string, key: string) => useQuery(settingBlobQueryOptions(playerId, key))
