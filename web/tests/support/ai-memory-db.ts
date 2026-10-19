import type { z } from 'zod'
import { createAiCredentialService, type AiCredentialRow, type AiCredentialServiceDb } from '@server/service/domain/ai/credential.service'
import { createAiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { createAiResultService, type AiResultRow, type AiResultServiceDb } from '@server/service/domain/ai/result.service'
import {
    createAiSettingsService,
    type AiOperatorLimitsRow,
    type AiSettingsServiceDb,
    type AiUserSettingRow,
} from '@server/service/domain/ai/settings.service'
import { createAiUsageService, type AiTokenTotals, type AiUsageServiceDb } from '@server/service/domain/ai/usage.service'
import { loadSecretBox } from '@server/service/shared/secret-box/secret-box'
import { createFailureDetail } from '@server/service/shared/ai-provider/http'
import type { AiFailureKind, AiProviderId, AiUsage } from '@server/dto/ai'
import type { AiGenerateResult, AiProviderAdapter, GenerateStructuredInput } from '@server/service/shared/ai-provider/types'

const credentialKey = (userId: string, provider: string) => `${userId}|${provider}`

export const createMemoryAiDb = () => {
    const credentials = new Map<string, AiCredentialRow & { userId: string }>()
    const usage = new Map<string, AiTokenTotals & { userId: string; day: string }>()
    const slots = new Map<string, { runId: string; expiresAt: number }>()
    const results = new Map<string, AiResultRow & { userId: string }>()
    const userSettings = new Map<string, AiUserSettingRow>()
    const operator: { row: (AiOperatorLimitsRow & { updatedBy: string }) | null } = { row: null }

    const credentialDb: AiCredentialServiceDb = {
        listCredentials: async (userId) => [...credentials.values()].filter((row) => row.userId === userId),
        findCredential: async ({ userId, provider }) => credentials.get(credentialKey(userId, provider)) ?? null,
        upsertCredential: async ({ userId, provider, sealed, keyHint, validatedAt, errorKind }) => {
            const existing = credentials.get(credentialKey(userId, provider))
            credentials.set(credentialKey(userId, provider), {
                userId,
                provider,
                ...sealed,
                keyHint,
                model: null,
                createdAt: existing?.createdAt ?? validatedAt,
                lastValidatedAt: validatedAt,
                lastErrorKind: errorKind,
            })
        },
        updateModel: async ({ userId, provider, model }) => {
            const row = credentials.get(credentialKey(userId, provider))
            if (!row) return false
            credentials.set(credentialKey(userId, provider), { ...row, model })
            return true
        },
        recordCheck: async ({ userId, provider, validatedAt, errorKind }) => {
            const row = credentials.get(credentialKey(userId, provider))
            if (!row) return
            credentials.set(credentialKey(userId, provider), {
                ...row,
                lastValidatedAt: validatedAt ?? row.lastValidatedAt,
                lastErrorKind: errorKind,
            })
        },
        rewrapCredential: async ({ userId, provider, previous, sealed }) => {
            const row = credentials.get(credentialKey(userId, provider))
            const isUnchanged = row?.kekVersion === previous.kekVersion && row.wrappedDek === previous.wrappedDek && row.dekIv === previous.dekIv
            if (!row || !isUnchanged) return
            credentials.set(credentialKey(userId, provider), { ...row, ...sealed })
        },
        deleteCredential: async ({ userId, provider }) => credentials.delete(credentialKey(userId, provider)),
    }

    const usageDb: AiUsageServiceDb = {
        sumUsage: async ({ userId, fromDay, toDay }) =>
            [...usage.values()]
                .filter((row) => row.userId === userId && row.day >= fromDay && row.day <= toDay)
                .reduce(
                    (total, row) => ({
                        calls: total.calls + row.calls,
                        inputTokens: total.inputTokens + row.inputTokens,
                        outputTokens: total.outputTokens + row.outputTokens,
                        cachedInputTokens: total.cachedInputTokens + row.cachedInputTokens,
                    }),
                    { calls: 0, inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 },
                ),
        reserveDailyCall: async ({ userId, day, limit }) => {
            const row = usage.get(credentialKey(userId, day)) ?? { userId, day, calls: 0, inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 }
            if (row.calls >= limit) return false
            usage.set(credentialKey(userId, day), { ...row, calls: row.calls + 1 })
            return true
        },
        addTokens: async ({ userId, day, usage: added }) => {
            const row = usage.get(credentialKey(userId, day)) ?? { userId, day, calls: 0, inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 }
            usage.set(credentialKey(userId, day), {
                ...row,
                inputTokens: row.inputTokens + added.inputTokens,
                outputTokens: row.outputTokens + added.outputTokens,
                cachedInputTokens: row.cachedInputTokens + added.cachedInputTokens,
            })
        },
        acquireSlot: async ({ userId, slot, runId, now, expiresAt }) => {
            const existing = slots.get(credentialKey(userId, String(slot)))
            if (existing && existing.expiresAt > now) return false
            slots.set(credentialKey(userId, String(slot)), { runId, expiresAt })
            return true
        },
        releaseSlot: async ({ userId, slot, runId }) => {
            if (slots.get(credentialKey(userId, String(slot)))?.runId === runId) slots.delete(credentialKey(userId, String(slot)))
        },
    }

    const resultDb: AiResultServiceDb = {
        findByInput: async ({ userId, feature, subjectKey, inputHash }) => results.get([userId, feature, subjectKey, inputHash].join('|')) ?? null,
        findLatest: async ({ userId, feature, subjectKey }) =>
            [...results.values()]
                .filter((row) => row.userId === userId && row.feature === feature && row.subjectKey === subjectKey)
                .toSorted((left, right) => right.createdAt - left.createdAt)[0] ?? null,
        upsert: async (row) => {
            results.set([row.userId, row.feature, row.subjectKey, row.inputHash].join('|'), row)
        },
    }

    const settingsDb: AiSettingsServiceDb = {
        getOperatorLimits: async () => operator.row,
        putOperatorLimits: async (row) => {
            operator.row = row
        },
        getUserSetting: async (userId) => userSettings.get(userId) ?? null,
        putUserSetting: async ({ userId, ...row }) => {
            userSettings.set(userId, row)
        },
    }

    return { credentialDb, usageDb, resultDb, settingsDb, credentials, usage, slots, results }
}

export type FakeAdapterOptions = {
    validKeys: Set<string>
    generate?: (input: GenerateStructuredInput<z.ZodType>) => { data: unknown; usage: AiUsage } | { failure: AiFailureKind; usage: AiUsage | null }
}

export const FAKE_USAGE = { inputTokens: 120, outputTokens: 40, cachedInputTokens: 20 }

/** A provider adapter that never leaves the process, recording each key it was given so tests can see where a key went. */
export const createFakeAdapter = (id: AiProviderId, options: FakeAdapterOptions) => {
    const seenKeys: string[] = []
    const generateCalls: GenerateStructuredInput<z.ZodType>[] = []
    const check = (apiKey: string) => {
        seenKeys.push(apiKey)
        return options.validKeys.has(apiKey)
    }
    const adapter: AiProviderAdapter = {
        id,
        strictness: 'native',
        validateKey: async ({ apiKey }) => (check(apiKey) ? { ok: true } : { ok: false, ...createFailureDetail('invalid_key', { httpStatus: 401 }) }),
        listModels: async ({ apiKey }) =>
            check(apiKey)
                ? { ok: true, models: [{ id: `${id}-model`, label: `${id} model`, structured: true }] }
                : { ok: false, ...createFailureDetail('invalid_key', { httpStatus: 401 }) },
        generateStructured: async <TSchema extends z.ZodType>(
            input: GenerateStructuredInput<TSchema>,
        ): Promise<AiGenerateResult<z.infer<TSchema>>> => {
            generateCalls.push(input)
            if (!check(input.apiKey)) return { ok: false, usage: null, ...createFailureDetail('invalid_key', { httpStatus: 401 }) }
            const outcome = options.generate?.(input) ?? { data: {}, usage: FAKE_USAGE }
            if ('failure' in outcome) return { ok: false, usage: outcome.usage, ...createFailureDetail(outcome.failure) }
            const parsed = input.schema.safeParse(outcome.data)
            if (!parsed.success) return { ok: false, usage: outcome.usage, ...createFailureDetail('invalid_output') }
            return { ok: true, data: parsed.data, usage: outcome.usage, requestId: 'req_fake' }
        },
    }
    return { adapter, seenKeys, generateCalls }
}

export const TEST_KEKS = {
    v1: Buffer.alloc(32, 1).toString('base64'),
    v2: Buffer.alloc(32, 2).toString('base64'),
}

export const testKekEnv = (current: keyof typeof TEST_KEKS, versions: (keyof typeof TEST_KEKS)[] = [current]) => ({
    AI_CREDENTIAL_KEKS: JSON.stringify(Object.fromEntries(versions.map((version) => [version, TEST_KEKS[version]]))),
    AI_CREDENTIAL_KEK_CURRENT: current,
})

type AiDbs = Pick<ReturnType<typeof createMemoryAiDb>, 'credentialDb' | 'usageDb' | 'resultDb' | 'settingsDb'>

type AiServicesOptions = {
    adapters: Partial<Record<AiProviderId, AiProviderAdapter>>
    env: () => Parameters<typeof loadSecretBox>[0]
    now?: () => number
}

/** The AI services wired to the given databases, fake adapters and a switchable key environment. */
export const createAiServices = <TDbs extends AiDbs>(dbs: TDbs, { adapters, env, now = () => Date.UTC(2026, 9, 19, 12) }: AiServicesOptions) => {
    let counter = 0
    const newId = (prefix: string) => `${prefix}${(++counter).toString(16).padStart(30, '0')}`
    const getProvider = (id: AiProviderId) => adapters[id] ?? createFakeAdapter(id, { validKeys: new Set() }).adapter
    const aiCredentialService = createAiCredentialService({ db: dbs.credentialDb, getProvider, getSecretBox: () => loadSecretBox(env()), now })
    const aiUsageService = createAiUsageService({ db: dbs.usageDb, now, newRunId: () => newId('run_') })
    const aiResultService = createAiResultService({ db: dbs.resultDb, newId, now })
    const aiSettingsService = createAiSettingsService({ db: dbs.settingsDb, now })
    const aiFeatureRunner = createAiFeatureRunner({
        credentialService: aiCredentialService,
        usageService: aiUsageService,
        resultService: aiResultService,
        settingsService: aiSettingsService,
        getProvider,
        now,
    })
    return { aiCredentialService, aiUsageService, aiResultService, aiSettingsService, aiFeatureRunner }
}

export const createMemoryAiServices = (options: AiServicesOptions) => {
    const memory = createMemoryAiDb()
    return { memory, ...createAiServices(memory, options) }
}
