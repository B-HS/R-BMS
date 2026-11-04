import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchEnvelope, fetchNoContent } from '@shared/lib/fetch'
import type {
    AiCredential,
    AiGoal,
    AiGoalInput,
    AiGoalPlanView,
    AiJudgeHabitView,
    AiModel,
    AiOperatorLimits,
    AiOperatorLimitsInput,
    AiPracticeFolderCreateInput,
    AiPracticeFolderResult,
    AiPracticeFolderView,
    AiPracticeSourceInput,
    AiProviderId,
    AiRecommendView,
    AiRecordQueryAnswer,
    AiRecordQueryPlan,
    AiUsageSummary,
    AiUserSetting,
    AiUserSettingInput,
    AiWeeklyView,
    AiCoachView,
    AiCompareQuery,
    AiCompareView,
    AiGaugeAdviceView,
    AiReportView,
    AiShareSetting,
    AiTrainingChartDetail,
    AiTrainingChartList,
    AiTrainingCreateInput,
    AiTrainingRemoval,
    AiTrainingRemoved,
    AiWeaknessProfile,
} from '@entities/ai.type'
import {
    AI_DENSITY_PARTS,
    AI_HAND_KEYS,
    AI_LANE_NAMES,
    AI_SOFLAN_PHASES,
    isAiPatternTag,
    isAiTrainingEdgeKey,
    splitAiWeaknessKey,
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

const AI_DIGEST = {
    RECOMMEND: 'recommend',
    PRACTICE: 'practice',
    GOAL_PLAN: 'goal_plan',
    WEEKLY: 'weekly',
    JUDGE_HABIT: 'judge_habit',
    REPORT: 'play_report',
    COACH: 'replay_coach',
    COMPARE: 'play_compare',
    GAUGE_ADVICE: 'gauge_advice',
} as const

const AI_FEATURE_PATH = {
    RECOMMEND: '/fe/me/ai/recommend',
    PRACTICE: '/fe/me/ai/practice-folder',
    GOALS: '/fe/me/ai/goals',
    WEEKLY: '/fe/me/ai/weekly',
    JUDGE_HABIT: '/fe/me/ai/judge-habit',
    REPORT: '/fe/me/ai/report',
    GAUGE_ADVICE: '/fe/me/ai/gauge-advice',
    PROFILE: '/fe/me/ai/profile',
    SHARING: '/fe/me/traces/sharing',
    TRAINING: '/fe/me/ai/training-charts',
} as const

const playPath = (scoreId: string, feature: 'coach' | 'compare') => `/fe/me/ai/plays/${encodeURIComponent(scoreId)}/${feature}`

const goalPath = (goalId: string) => `${AI_FEATURE_PATH.GOALS}/${encodeURIComponent(goalId)}`

export const aiRecommendQueryOptions = (tableId: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.RECOMMEND, { table: tableId }),
        queryFn: () => fetchEnvelope<AiRecommendView>(buildApiUrl(AI_FEATURE_PATH.RECOMMEND, { table: tableId })),
    })

export const aiPracticeFolderQueryOptions = (source: AiPracticeSourceInput) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.PRACTICE, source),
        queryFn: () => fetchEnvelope<AiPracticeFolderView>(buildApiUrl(AI_FEATURE_PATH.PRACTICE, source)),
    })

export const aiGoalListQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.GOALS,
        queryFn: () => fetchEnvelope<AiGoal[]>(buildApiUrl(AI_FEATURE_PATH.GOALS)),
    })

export const aiGoalPlanQueryOptions = (goalId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.GOAL_PLAN, { goalId }),
        queryFn: () => fetchEnvelope<AiGoalPlanView>(buildApiUrl(`${goalPath(goalId)}/plan`)),
    })

export const aiWeeklyQueryOptions = (week: string | null) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.WEEKLY, { week }),
        queryFn: () => fetchEnvelope<AiWeeklyView>(buildApiUrl(AI_FEATURE_PATH.WEEKLY, { week })),
    })

export const aiJudgeHabitQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.JUDGE_HABIT, {}),
        queryFn: () => fetchEnvelope<AiJudgeHabitView>(buildApiUrl(AI_FEATURE_PATH.JUDGE_HABIT)),
    })

export const aiReportQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.REPORT, {}),
        queryFn: () => fetchEnvelope<AiReportView>(buildApiUrl(AI_FEATURE_PATH.REPORT)),
    })

export const aiCoachQueryOptions = (scoreId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.COACH, { scoreId }),
        queryFn: () => fetchEnvelope<AiCoachView>(buildApiUrl(playPath(scoreId, 'coach'))),
    })

export const aiCompareQueryOptions = (scoreId: string, query: AiCompareQuery) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.COMPARE, { scoreId, ...query }),
        queryFn: () => fetchEnvelope<AiCompareView>(buildApiUrl(playPath(scoreId, 'compare'), query)),
    })

export const aiGaugeAdviceQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.DIGEST(AI_DIGEST.GAUGE_ADVICE, {}),
        queryFn: () => fetchEnvelope<AiGaugeAdviceView>(buildApiUrl(AI_FEATURE_PATH.GAUGE_ADVICE)),
    })

/** The weakness profile (plan 2026-11-04 2.5.6): computed by the server from the analysed plays, with no AI call. */
export const aiProfileQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.PROFILE,
        queryFn: () => fetchEnvelope<AiWeaknessProfile>(buildApiUrl(AI_FEATURE_PATH.PROFILE)),
    })

const trainingPath = (chartId: string) => `${AI_FEATURE_PATH.TRAINING}/${encodeURIComponent(chartId)}`

/** The own training charts, newest first, with the room left and whether an AI key is registered (plan 2026-11-04 7.2). */
export const aiTrainingListQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.TRAINING_LIST,
        queryFn: () => fetchEnvelope<AiTrainingChartList>(buildApiUrl(AI_FEATURE_PATH.TRAINING)),
    })

export const aiTrainingDetailQueryOptions = (chartId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.TRAINING_DETAIL(chartId),
        queryFn: () => fetchEnvelope<AiTrainingChartDetail>(buildApiUrl(trainingPath(chartId))),
    })

/** What deleting one chart takes away and leaves, counted by the server when it is asked. */
export const aiTrainingRemovalQueryOptions = (chartId: string) =>
    queryOptions({
        queryKey: QUERY_KEY.AI.TRAINING_REMOVAL(chartId),
        queryFn: () => fetchEnvelope<AiTrainingRemoval>(buildApiUrl(`${trainingPath(chartId)}/removal`)),
        staleTime: 0,
        gcTime: 0,
    })

export const aiShareSettingQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.AI.SHARING,
        queryFn: () => fetchEnvelope<AiShareSetting>(buildApiUrl(AI_FEATURE_PATH.SHARING)),
    })

export const useAiReport = () => useQuery(aiReportQueryOptions())

export const useAiCoach = (scoreId: string) => useQuery(aiCoachQueryOptions(scoreId))

/** Keeps the last comparison of this play on screen while another target loads, so the target list does not disappear while switching. */
export const useAiCompare = (scoreId: string, query: AiCompareQuery) =>
    useQuery({
        ...aiCompareQueryOptions(scoreId, query),
        placeholderData: (previous) => (previous?.base.score_id === scoreId ? previous : undefined),
    })

export const useAiGaugeAdvice = () => useQuery(aiGaugeAdviceQueryOptions())

export const useAiShareSetting = () => useQuery(aiShareSettingQueryOptions())

export const useAiProfile = (enabled = true) => useQuery({ ...aiProfileQueryOptions(), enabled })

export const useAiTrainingList = (enabled = true) => useQuery({ ...aiTrainingListQueryOptions(), enabled })

export const useAiTrainingDetail = (chartId: string) => useQuery(aiTrainingDetailQueryOptions(chartId))

export const useAiTrainingRemoval = (chartId: string, enabled: boolean) => useQuery({ ...aiTrainingRemovalQueryOptions(chartId), enabled })

const isOneOf = <T extends string>(values: readonly T[], value: string | undefined): value is T => values.some((entry) => entry === value)
const SCRATCH_LANE = AI_LANE_NAMES[AI_LANE_NAMES.length - 1]

/** Names a weakness key for the screen (`tag:trill` → 트릴, `combo:a+b` → a + b); a key outside the grammar is shown as it is. */
export const useAiWeaknessLabel = () => {
    const t = useTranslations('pages.ai')
    const tagName = (tag: string | undefined) => (tag !== undefined && isAiPatternTag(tag) ? t(`patternTag.${tag}`) : (tag ?? ''))

    return (key: string) => {
        const split = splitAiWeaknessKey(key)
        if (!split) return key
        const [first, second] = split.parts
        if (split.kind === 'tag') return tagName(first)
        if (split.kind === 'solo') return t('weaknessKind.solo', { tag: tagName(first) })
        if (split.kind === 'combo') return t('weaknessKind.combo', { first: tagName(first), second: tagName(second) })
        if (split.kind === 'timing') return t('weaknessKind.timing', { tag: tagName(first) })
        if (split.kind === 'bp_context') return t('weaknessKind.bp_context', { tag: tagName(first) })
        if (split.kind === 'gauge_context') return t('weaknessKind.gauge_context', { tag: tagName(first) })
        if (split.kind === 'lane') return first === SCRATCH_LANE ? t('weaknessKind.laneScratch') : t('weaknessKind.lane', { lane: first ?? '' })
        if (split.kind === 'hand') return isOneOf(AI_HAND_KEYS, first) ? t(`weaknessKind.hand.${first}`) : key
        if (split.kind === 'bpm') return t('weaknessKind.bpm', { band: first ?? '' })
        if (split.kind === 'soflan') return isOneOf(AI_SOFLAN_PHASES, first) ? t(`weaknessKind.soflan.${first}`) : key
        if (split.kind === 'density') return isOneOf(AI_DENSITY_PARTS, first) ? t(`weaknessKind.density.${first}`) : key
        return t('weaknessKind.fatigue')
    }
}

/** Names a section's focus key: the warm-up and the cool-down by their own words, any other key like `useAiWeaknessLabel`. */
export const useAiTrainingFocusLabel = () => {
    const t = useTranslations('pages.ai.training')
    const weaknessLabel = useAiWeaknessLabel()

    return (key: string) => (isAiTrainingEdgeKey(key) ? t(`edge.${key}`) : weaknessLabel(key))
}

export const useAiRecommend = (tableId: string | null) => useQuery(aiRecommendQueryOptions(tableId))

export const useAiPracticeFolder = (source: AiPracticeSourceInput, enabled = true) => useQuery({ ...aiPracticeFolderQueryOptions(source), enabled })

export const useAiGoalList = () => useQuery(aiGoalListQueryOptions())

export const useAiGoalPlan = (goalId: string) => useQuery(aiGoalPlanQueryOptions(goalId))

export const useAiWeekly = (week: string | null) => useQuery(aiWeeklyQueryOptions(week))

export const useAiJudgeHabit = () => useQuery(aiJudgeHabitQueryOptions())

/**
 * One press of an "AI 해석 만들기" button: a single POST, then the usage and the feature's digest are read again so the stored
 * answer shows. A failure is shown on the panel with its wait time and settings link, so the shared error toast is skipped.
 */
const useAiInterpretMutation = <TInput, TResult>(options: { path: (input: TInput) => string; feature: string }) => {
    const queryClient = useQueryClient()
    return useMutation({
        meta: { suppressErrorToast: true },
        mutationFn: (input: TInput) =>
            fetchEnvelope<TResult>(buildApiUrl(options.path(input)), {
                method: 'POST',
                headers: JSON_HEADERS,
                body: JSON.stringify(input ?? {}),
            }),
        onSettled: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.USAGE }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.DIGEST_FEATURE(options.feature) }),
            ]),
    })
}

export const useInterpretAiRecommend = () =>
    useAiInterpretMutation<{ table?: string }, NonNullable<AiRecommendView['ai']>>({
        path: () => AI_FEATURE_PATH.RECOMMEND,
        feature: AI_DIGEST.RECOMMEND,
    })

export const usePreviewAiPracticeFolder = () =>
    useAiInterpretMutation<AiPracticeSourceInput, NonNullable<AiPracticeFolderView['ai']>>({
        path: () => `${AI_FEATURE_PATH.PRACTICE}/preview`,
        feature: AI_DIGEST.PRACTICE,
    })

export const useInterpretAiGoalPlan = () =>
    useAiInterpretMutation<{ goalId: string }, NonNullable<AiGoalPlanView['ai']>>({
        path: ({ goalId }) => `${goalPath(goalId)}/plan`,
        feature: AI_DIGEST.GOAL_PLAN,
    })

export const useInterpretAiWeekly = () =>
    useAiInterpretMutation<{ week?: string }, NonNullable<AiWeeklyView['ai']>>({
        path: () => AI_FEATURE_PATH.WEEKLY,
        feature: AI_DIGEST.WEEKLY,
    })

export const useInterpretAiJudgeHabit = () =>
    useAiInterpretMutation<Record<string, never>, NonNullable<AiJudgeHabitView['ai']>>({
        path: () => AI_FEATURE_PATH.JUDGE_HABIT,
        feature: AI_DIGEST.JUDGE_HABIT,
    })

export const useInterpretAiReport = () =>
    useAiInterpretMutation<Record<string, never>, NonNullable<AiReportView['ai']>>({
        path: () => AI_FEATURE_PATH.REPORT,
        feature: AI_DIGEST.REPORT,
    })

export const useInterpretAiCoach = (scoreId: string) =>
    useAiInterpretMutation<Record<string, never>, NonNullable<AiCoachView['ai']>>({
        path: () => playPath(scoreId, 'coach'),
        feature: AI_DIGEST.COACH,
    })

export const useInterpretAiCompare = (scoreId: string) =>
    useAiInterpretMutation<AiCompareQuery, NonNullable<AiCompareView['ai']>>({
        path: () => playPath(scoreId, 'compare'),
        feature: AI_DIGEST.COMPARE,
    })

export const useInterpretAiGaugeAdvice = () =>
    useAiInterpretMutation<Record<string, never>, NonNullable<AiGaugeAdviceView['ai']>>({
        path: () => AI_FEATURE_PATH.GAUGE_ADVICE,
        feature: AI_DIGEST.GAUGE_ADVICE,
    })

/** Turns sharing segment totals with rivals on or off; the comparisons that read it are read again. */
export const useSaveAiShareSetting = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (rivalSegments: boolean) =>
            fetchEnvelope<AiShareSetting>(buildApiUrl(AI_FEATURE_PATH.SHARING), {
                method: 'PUT',
                headers: JSON_HEADERS,
                body: JSON.stringify({ rival_segments: rivalSegments }),
            }),
        onSuccess: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.SHARING }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.DIGEST_FEATURE(AI_DIGEST.COMPARE) }),
            ]),
    })
}

/** Makes (or, for the same preview, updates) the practice folder the user confirmed; it shows in the game's MY FOLDERS on the next refresh. */
export const useCreateAiPracticeFolder = () => {
    const queryClient = useQueryClient()
    const t = useTranslations('pages.ai.practiceFolder')
    return useMutation({
        mutationFn: (input: AiPracticeFolderCreateInput) =>
            fetchEnvelope<AiPracticeFolderResult>(buildApiUrl(AI_FEATURE_PATH.PRACTICE), {
                method: 'POST',
                headers: JSON_HEADERS,
                body: JSON.stringify(input),
            }),
        onSuccess: async (result) => {
            await Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.ALL }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.DIGEST_FEATURE(AI_DIGEST.PRACTICE) }),
            ])
            toast.success(t(result.outcome, { name: result.name }))
        },
    })
}

export const useCreateAiGoal = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (input: AiGoalInput) =>
            fetchEnvelope<AiGoal>(buildApiUrl(AI_FEATURE_PATH.GOALS), { method: 'POST', headers: JSON_HEADERS, body: JSON.stringify(input) }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.GOALS }),
    })
}

export const useArchiveAiGoal = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: (goalId: string) => fetchNoContent(buildApiUrl(goalPath(goalId)), { method: 'DELETE' }),
        onSuccess: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.GOALS }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.DIGEST_FEATURE(AI_DIGEST.GOAL_PLAN), refetchType: 'none' }),
            ]),
    })
}

/**
 * One press of "만들기": a single POST that asks the provider once and lays the chart out on the server, so it can take minutes. A
 * failure (no key, the 20-chart limit, a full folder list, too few plays) is worded on the panel, so the shared error toast is
 * skipped. The AI usage, the chart list and the folders (the chart joins the "훈련곡" folder) are read again.
 */
export const useCreateAiTrainingChart = () => {
    const queryClient = useQueryClient()
    const t = useTranslations('pages.ai.training')
    return useMutation({
        meta: { suppressErrorToast: true },
        mutationFn: (input: AiTrainingCreateInput) =>
            fetchEnvelope<AiTrainingChartDetail>(buildApiUrl(AI_FEATURE_PATH.TRAINING), {
                method: 'POST',
                headers: JSON_HEADERS,
                body: JSON.stringify(input),
            }),
        onSuccess: (chart) => toast.success(t('created', { title: chart.title })),
        onSettled: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.USAGE }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.TRAINING }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.ALL }),
            ]),
    })
}

/** Deletes one own training chart; its folder items go with it, and the plays, traces and replays stay. */
export const useRemoveAiTrainingChart = () => {
    const queryClient = useQueryClient()
    const t = useTranslations('pages.ai.training')
    return useMutation({
        mutationFn: (chartId: string) => fetchEnvelope<AiTrainingRemoved>(buildApiUrl(trainingPath(chartId)), { method: 'DELETE' }),
        onSuccess: async (_, chartId) => {
            await Promise.all([
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.TRAINING_DETAIL(chartId), refetchType: 'none' }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.AI.TRAINING_LIST }),
                queryClient.invalidateQueries({ queryKey: QUERY_KEY.FOLDER.ALL }),
            ])
            toast.success(t('removed'))
        },
    })
}
