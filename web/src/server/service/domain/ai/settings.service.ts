import { AI_DEFAULT_LIMITS, type AiOperatorLimitsPutInput, type AiProviderId, type AiUserSettingPutInput } from '@server/dto/ai'

export type AiOperatorLimitsRow = {
    dailyCalls: number
    concurrentRuns: number
    defaultMonthlyTokenLimit: number | null
    updatedAt: number
}

export type AiUserSettingRow = {
    monthlyTokenLimit: number | null
    defaultProvider: AiProviderId | null
    updatedAt: number
}

export type AiSettingsServiceDb = {
    getOperatorLimits: () => Promise<AiOperatorLimitsRow | null>
    putOperatorLimits: (row: AiOperatorLimitsRow & { updatedBy: string }) => Promise<void>
    getUserSetting: (userId: string) => Promise<AiUserSettingRow | null>
    putUserSetting: (row: AiUserSettingRow & { userId: string }) => Promise<void>
}

type AiSettingsServiceDeps = {
    db: AiSettingsServiceDb
    now: () => number
}

const toOperatorLimitsOutput = (row: AiOperatorLimitsRow | null) => ({
    daily_calls: row?.dailyCalls ?? AI_DEFAULT_LIMITS.DAILY_CALLS,
    concurrent_runs: row?.concurrentRuns ?? AI_DEFAULT_LIMITS.CONCURRENT_RUNS,
    default_monthly_token_limit: row ? row.defaultMonthlyTokenLimit : AI_DEFAULT_LIMITS.DEFAULT_MONTHLY_TOKEN_LIMIT,
    updated_at: row?.updatedAt ?? null,
})

const toUserSettingOutput = (row: AiUserSettingRow | null) => ({
    monthly_token_limit: row?.monthlyTokenLimit ?? null,
    default_provider: row?.defaultProvider ?? null,
    updated_at: row?.updatedAt ?? null,
})

export const createAiSettingsService = (deps: AiSettingsServiceDeps) => {
    const getOperatorLimits = async () => toOperatorLimitsOutput(await deps.db.getOperatorLimits())

    const getUserSetting = async (userId: string) => toUserSettingOutput(await deps.db.getUserSetting(userId))

    return {
        getOperatorLimits,

        putOperatorLimits: async ({ userId, input }: { userId: string; input: AiOperatorLimitsPutInput }) => {
            const row = {
                dailyCalls: input.daily_calls,
                concurrentRuns: input.concurrent_runs,
                defaultMonthlyTokenLimit: input.default_monthly_token_limit,
                updatedAt: deps.now(),
            }
            await deps.db.putOperatorLimits({ ...row, updatedBy: userId })
            return toOperatorLimitsOutput(row)
        },

        getUserSetting,

        putUserSetting: async ({ userId, input }: { userId: string; input: AiUserSettingPutInput }) => {
            const row = { monthlyTokenLimit: input.monthly_token_limit, defaultProvider: input.default_provider, updatedAt: deps.now() }
            await deps.db.putUserSetting({ ...row, userId })
            return toUserSettingOutput(row)
        },

        /** The limits one user's AI calls run under: the operator's values, with the user's own monthly token cap first. */
        resolveLimits: async (userId: string) => {
            const [operator, user] = await Promise.all([getOperatorLimits(), getUserSetting(userId)])
            return {
                dailyCalls: operator.daily_calls,
                concurrentRuns: operator.concurrent_runs,
                monthlyTokenLimit: user.monthly_token_limit ?? operator.default_monthly_token_limit,
            }
        },
    }
}

export type AiSettingsService = ReturnType<typeof createAiSettingsService>
export type AiEffectiveLimits = Awaited<ReturnType<AiSettingsService['resolveLimits']>>
