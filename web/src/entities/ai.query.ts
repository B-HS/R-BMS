import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchNoContent } from '@shared/lib/fetch'
import type {
    AiCredential,
    AiModel,
    AiOperatorLimits,
    AiOperatorLimitsInput,
    AiProviderId,
    AiRecordQueryAnswer,
    AiRecordQueryPlan,
    AiUsageSummary,
    AiUserSetting,
    AiUserSettingInput,
} from '@entities/ai.type'

const MODEL_LIST_STALE_TIME_MS = 300_000
const JSON_HEADERS = { 'content-type': 'application/json' }

const credentialPath = (provider: AiProviderId) => `/fe/me/ai/credentials/${encodeURIComponent(provider)}`

export const aiCredentialListQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.CREDENTIALS,
        queryFn: () => fetchEnvelope<AiCredential[]>(buildApiUrl('/fe/me/ai/credentials')),
    })

export const aiModelListQueryOptions = (provider: AiProviderId) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.MODELS(provider),
        queryFn: () => fetchEnvelope<AiModel[]>(buildApiUrl(`${credentialPath(provider)}/models`)),
        staleTime: MODEL_LIST_STALE_TIME_MS,
        retry: false,
    })

export const aiUsageSummaryQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.USAGE,
        queryFn: () => fetchEnvelope<AiUsageSummary>(buildApiUrl('/fe/me/ai/usage')),
    })

export const aiUserSettingQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.SETTINGS,
        queryFn: () => fetchEnvelope<AiUserSetting>(buildApiUrl('/fe/me/ai/settings')),
    })

export const aiOperatorLimitsQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.ADMIN.AI_LIMITS,
        queryFn: () => fetchEnvelope<AiOperatorLimits>(buildApiUrl('/admin/ai/limits')),
    })

export const useAiCredentialList = () => useQuery(aiCredentialListQueryOptions())

export const useAiModelList = (provider: AiProviderId, enabled: boolean) => useQuery({ ...aiModelListQueryOptions(provider), enabled })

export const useAiUsageSummary = () => useQuery(aiUsageSummaryQueryOptions())

export const useAiUserSetting = () => useQuery(aiUserSettingQueryOptions())

export const useAiOperatorLimits = () => useQuery(aiOperatorLimitsQueryOptions())

/** `gcTime: 0` drops the finished mutation, and the plain API key in its variables, as soon as the caller resets it. */
export const useRegisterAiCredential = () => {
    const queryClient = useQueryClient()
    return useMutation({
        gcTime: 0,
        mutationFn: ({ provider, apiKey }: { provider: AiProviderId; apiKey: string }) =>
            fetchEnvelope<AiCredential>(buildApiUrl(credentialPath(provider)), {
                method: 'PUT',
                headers: JSON_HEADERS,
                body: JSON.stringify({ api_key: apiKey }),
            }),
        onSuccess: (_, { provider }) =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.CREDENTIALS }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.MODELS(provider) }),
            ]),
    })
}

export const useRevalidateAiCredential = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (provider: AiProviderId) => fetchEnvelope<AiCredential>(buildApiUrl(`${credentialPath(provider)}/validate`), { method: 'POST' }),
        onSettled: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.CREDENTIALS }),
    })
}

export const useSetAiCredentialModel = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: ({ provider, model }: { provider: AiProviderId; model: string }) =>
            fetchEnvelope<AiCredential>(buildApiUrl(credentialPath(provider)), {
                method: 'PATCH',
                headers: JSON_HEADERS,
                body: JSON.stringify({ model }),
            }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.CREDENTIALS }),
    })
}

export const useRemoveAiCredential = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (provider: AiProviderId) => fetchNoContent(buildApiUrl(credentialPath(provider)), { method: 'DELETE' }),
        onSuccess: (_, provider) =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.CREDENTIALS }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.MODELS(provider), refetchType: 'none' }),
            ]),
    })
}

export const useSaveAiUserSetting = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: AiUserSettingInput) =>
            fetchEnvelope<AiUserSetting>(buildApiUrl('/fe/me/ai/settings'), { method: 'PUT', headers: JSON_HEADERS, body: JSON.stringify(input) }),
        onSuccess: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.SETTINGS }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.USAGE }),
            ]),
    })
}

export const useSaveAiOperatorLimits = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: AiOperatorLimitsInput) =>
            fetchEnvelope<AiOperatorLimits>(buildApiUrl('/admin/ai/limits'), { method: 'PUT', headers: JSON_HEADERS, body: JSON.stringify(input) }),
        onSuccess: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.ADMIN.AI_LIMITS }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.USAGE }),
            ]),
    })
}

const RECORD_QUERY_PATH = '/fe/me/ai/record-query'

/**
 * One AI call that turns the question into filters; the answer also carries the rows when every filter passed the server check.
 * A failure is shown on the panel with its wait time and settings link, so the shared error toast is skipped.
 */
export const useAskAiRecordQuery = () => {
    const queryClient = useQueryClient()
    return useMutation({
        meta: { suppressErrorToast: true },
        mutationFn: (question: string) =>
            fetchEnvelope<AiRecordQueryAnswer>(buildApiUrl(RECORD_QUERY_PATH), {
                method: 'POST',
                headers: JSON_HEADERS,
                body: JSON.stringify({ question }),
            }),
        onSettled: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.USAGE }),
    })
}

/** Runs filters the user edited, with no AI call. */
export const useRunAiRecordQuery = () =>
    useMutation({
        mutationFn: (plan: AiRecordQueryPlan) =>
            fetchEnvelope<AiRecordQueryAnswer>(buildApiUrl(`${RECORD_QUERY_PATH}/run`), {
                method: 'POST',
                headers: JSON_HEADERS,
                body: JSON.stringify(plan),
            }),
    })
