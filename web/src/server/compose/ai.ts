import { and, desc, eq, gte, lt, lte, sql, sum, type SQL } from 'drizzle-orm'
import type { Database } from '@server/db'
import { aiCredential, aiInflight, aiResult, aiSetting, aiUsage, aiUserSetting } from '@server/db/schema'
import { aiFailureKindSchema, aiProviderIdSchema, aiUsageSchema } from '@server/dto/ai'
import { getEnv } from '@server/lib/env'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { summarizeError } from '@server/lib/error-summary'
import { newId } from '@server/lib/token'
import { createAiCredentialService, type AiCredentialServiceDb } from '@server/service/domain/ai/credential.service'
import { createAiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import { createAiResultService, type AiResultServiceDb } from '@server/service/domain/ai/result.service'
import { createAiSettingsService, type AiSettingsServiceDb } from '@server/service/domain/ai/settings.service'
import { createAiUsageService, type AiUsageServiceDb } from '@server/service/domain/ai/usage.service'
import { getAiProvider } from '@server/service/shared/ai-provider'
import { loadSecretBox } from '@server/service/shared/secret-box/secret-box'

const OPERATOR_SETTING_ID = 'global'
const RUN_ID_PREFIX = 'run_'
const EMPTY_USAGE = { inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 }

/**
 * Runs a write whose bound values include key material. A failed write is logged and thrown again as an error with no query and no
 * values, so neither the ciphertext nor the wrapped data key reaches a log or a response.
 */
const withoutBoundValues = async (operation: string, write: () => Promise<unknown>) => {
    try {
        await write()
    } catch (error) {
        console.error('AI credential write failed', { operation, ...summarizeError(error) })
        throw createAppError(ERROR_CODE.INTERNAL_ERROR, { reason: 'ai_credential_write_failed' })
    }
}

const toProvider = (value: string | null) => {
    const parsed = aiProviderIdSchema.safeParse(value)
    return parsed.success ? parsed.data : null
}

const toFailureKind = (value: string | null) => {
    const parsed = aiFailureKindSchema.safeParse(value)
    return parsed.success ? parsed.data : null
}

const toUsage = (value: unknown) => {
    const parsed = aiUsageSchema.safeParse(value)
    return parsed.success ? parsed.data : EMPTY_USAGE
}

const credentialSelection = {
    provider: aiCredential.provider,
    ciphertext: aiCredential.ciphertext,
    iv: aiCredential.iv,
    authTag: aiCredential.authTag,
    wrappedDek: aiCredential.wrappedDek,
    dekIv: aiCredential.dekIv,
    dekTag: aiCredential.dekTag,
    kekVersion: aiCredential.kekVersion,
    keyHint: aiCredential.keyHint,
    model: aiCredential.model,
    createdAt: aiCredential.createdAt,
    lastValidatedAt: aiCredential.lastValidatedAt,
    lastErrorKind: aiCredential.lastErrorKind,
}

const resultSelection = {
    id: aiResult.id,
    feature: aiResult.feature,
    subjectKey: aiResult.subjectKey,
    inputHash: aiResult.inputHash,
    analysisVersion: aiResult.analysisVersion,
    promptVersion: aiResult.promptVersion,
    provider: aiResult.provider,
    model: aiResult.model,
    output: aiResult.output,
    usage: aiResult.usage,
    createdAt: aiResult.createdAt,
}

export const createAiDbs = (db: Database) => {
    const selectCredentials = (where: SQL | undefined) => db.select(credentialSelection).from(aiCredential).where(where)

    const toCredentialRow = (row: Awaited<ReturnType<typeof selectCredentials>>[number]) => {
        const provider = toProvider(row.provider)
        if (!provider) return null
        return { ...row, provider, createdAt: row.createdAt.getTime(), lastErrorKind: toFailureKind(row.lastErrorKind) }
    }

    const selectResults = (where: SQL | undefined) => db.select(resultSelection).from(aiResult).where(where)

    const toResultRow = (row: Awaited<ReturnType<typeof selectResults>>[number] | undefined) => {
        const provider = row ? toProvider(row.provider) : null
        if (!row || !provider) return null
        return { ...row, provider, usage: toUsage(row.usage), createdAt: row.createdAt.getTime() }
    }

    const credentialDb: AiCredentialServiceDb = {
        listCredentials: async (userId) => {
            const rows = await selectCredentials(eq(aiCredential.userId, userId))
            return rows.flatMap((row) => toCredentialRow(row) ?? [])
        },

        findCredential: async ({ userId, provider }) => {
            const [row] = await selectCredentials(and(eq(aiCredential.userId, userId), eq(aiCredential.provider, provider))).limit(1)
            return row ? toCredentialRow(row) : null
        },

        upsertCredential: async ({ userId, provider, sealed, keyHint, validatedAt, errorKind, updatedAt }) => {
            const values = { ...sealed, keyHint, model: null, lastValidatedAt: validatedAt, lastErrorKind: errorKind, updatedAt }
            await withoutBoundValues('upsert', () =>
                db
                    .insert(aiCredential)
                    .values({ userId, provider, ...values })
                    .onDuplicateKeyUpdate({ set: values }),
            )
        },

        updateModel: async ({ userId, provider, model, updatedAt }) => {
            const [result] = await db
                .update(aiCredential)
                .set({ model, updatedAt })
                .where(and(eq(aiCredential.userId, userId), eq(aiCredential.provider, provider)))
            return result.affectedRows > 0
        },

        recordCheck: async ({ userId, provider, validatedAt, errorKind, updatedAt }) => {
            await db
                .update(aiCredential)
                .set({ ...(validatedAt === null ? {} : { lastValidatedAt: validatedAt }), lastErrorKind: errorKind, updatedAt })
                .where(and(eq(aiCredential.userId, userId), eq(aiCredential.provider, provider)))
        },

        rewrapCredential: async ({ userId, provider, previous, sealed }) => {
            await withoutBoundValues('rewrap', () =>
                db
                    .update(aiCredential)
                    .set({ wrappedDek: sealed.wrappedDek, dekIv: sealed.dekIv, dekTag: sealed.dekTag, kekVersion: sealed.kekVersion })
                    .where(
                        and(
                            eq(aiCredential.userId, userId),
                            eq(aiCredential.provider, provider),
                            eq(aiCredential.kekVersion, previous.kekVersion),
                            eq(aiCredential.wrappedDek, previous.wrappedDek),
                            eq(aiCredential.dekIv, previous.dekIv),
                        ),
                    ),
            )
        },

        deleteCredential: async ({ userId, provider }) => {
            const [result] = await db.delete(aiCredential).where(and(eq(aiCredential.userId, userId), eq(aiCredential.provider, provider)))
            return result.affectedRows > 0
        },
    }

    const usageDb: AiUsageServiceDb = {
        sumUsage: async ({ userId, fromDay, toDay }) => {
            const [row] = await db
                .select({
                    calls: sum(aiUsage.calls).mapWith(Number),
                    inputTokens: sum(aiUsage.inputTokens).mapWith(Number),
                    outputTokens: sum(aiUsage.outputTokens).mapWith(Number),
                    cachedInputTokens: sum(aiUsage.cachedInputTokens).mapWith(Number),
                })
                .from(aiUsage)
                .where(and(eq(aiUsage.userId, userId), gte(aiUsage.day, fromDay), lte(aiUsage.day, toDay)))
            return {
                calls: row?.calls ?? 0,
                inputTokens: row?.inputTokens ?? 0,
                outputTokens: row?.outputTokens ?? 0,
                cachedInputTokens: row?.cachedInputTokens ?? 0,
            }
        },

        reserveDailyCall: async ({ userId, day, limit }) => {
            await db.insert(aiUsage).ignore().values({ userId, day })
            const [result] = await db
                .update(aiUsage)
                .set({ calls: sql`${aiUsage.calls} + 1` })
                .where(and(eq(aiUsage.userId, userId), eq(aiUsage.day, day), lt(aiUsage.calls, limit)))
            return result.affectedRows > 0
        },

        addTokens: async ({ userId, day, usage }) => {
            await db
                .insert(aiUsage)
                .values({ userId, day, inputTokens: usage.inputTokens, outputTokens: usage.outputTokens, cachedInputTokens: usage.cachedInputTokens })
                .onDuplicateKeyUpdate({
                    set: {
                        inputTokens: sql`${aiUsage.inputTokens} + ${usage.inputTokens}`,
                        outputTokens: sql`${aiUsage.outputTokens} + ${usage.outputTokens}`,
                        cachedInputTokens: sql`${aiUsage.cachedInputTokens} + ${usage.cachedInputTokens}`,
                    },
                })
        },

        acquireSlot: async ({ userId, slot, runId, now: at, expiresAt }) => {
            const [inserted] = await db.insert(aiInflight).ignore().values({ userId, slot, runId, expiresAt })
            if (inserted.affectedRows > 0) return true
            const [taken] = await db
                .update(aiInflight)
                .set({ runId, expiresAt })
                .where(and(eq(aiInflight.userId, userId), eq(aiInflight.slot, slot), lte(aiInflight.expiresAt, at)))
            return taken.affectedRows > 0
        },

        releaseSlot: async ({ userId, slot, runId }) => {
            await db.delete(aiInflight).where(and(eq(aiInflight.userId, userId), eq(aiInflight.slot, slot), eq(aiInflight.runId, runId)))
        },
    }

    const resultDb: AiResultServiceDb = {
        findByInput: async ({ userId, feature, subjectKey, inputHash }) => {
            const [row] = await selectResults(
                and(
                    eq(aiResult.userId, userId),
                    eq(aiResult.feature, feature),
                    eq(aiResult.subjectKey, subjectKey),
                    eq(aiResult.inputHash, inputHash),
                ),
            ).limit(1)
            return toResultRow(row)
        },

        findLatest: async ({ userId, feature, subjectKey }) => {
            const [row] = await selectResults(and(eq(aiResult.userId, userId), eq(aiResult.feature, feature), eq(aiResult.subjectKey, subjectKey)))
                .orderBy(desc(aiResult.createdAt))
                .limit(1)
            return toResultRow(row)
        },

        upsert: async (row) => {
            const values = {
                analysisVersion: row.analysisVersion,
                promptVersion: row.promptVersion,
                provider: row.provider,
                model: row.model,
                output: row.output,
                usage: row.usage,
                createdAt: new Date(row.createdAt),
            }
            await db
                .insert(aiResult)
                .values({ id: row.id, userId: row.userId, feature: row.feature, subjectKey: row.subjectKey, inputHash: row.inputHash, ...values })
                .onDuplicateKeyUpdate({ set: values })
        },
    }

    const settingsDb: AiSettingsServiceDb = {
        getOperatorLimits: async () => {
            const [row] = await db
                .select({
                    dailyCalls: aiSetting.dailyCalls,
                    concurrentRuns: aiSetting.concurrentRuns,
                    defaultMonthlyTokenLimit: aiSetting.defaultMonthlyTokenLimit,
                    updatedAt: aiSetting.updatedAt,
                })
                .from(aiSetting)
                .where(eq(aiSetting.id, OPERATOR_SETTING_ID))
                .limit(1)
            return row ?? null
        },

        putOperatorLimits: async ({ dailyCalls, concurrentRuns, defaultMonthlyTokenLimit, updatedAt, updatedBy }) => {
            const values = { dailyCalls, concurrentRuns, defaultMonthlyTokenLimit, updatedAt, updatedBy }
            await db
                .insert(aiSetting)
                .values({ id: OPERATOR_SETTING_ID, ...values })
                .onDuplicateKeyUpdate({ set: values })
        },

        getUserSetting: async (userId) => {
            const [row] = await db
                .select({
                    monthlyTokenLimit: aiUserSetting.monthlyTokenLimit,
                    defaultProvider: aiUserSetting.defaultProvider,
                    updatedAt: aiUserSetting.updatedAt,
                })
                .from(aiUserSetting)
                .where(eq(aiUserSetting.userId, userId))
                .limit(1)
            return row ? { ...row, defaultProvider: toProvider(row.defaultProvider) } : null
        },

        putUserSetting: async ({ userId, monthlyTokenLimit, defaultProvider, updatedAt }) => {
            const values = { monthlyTokenLimit, defaultProvider, updatedAt }
            await db
                .insert(aiUserSetting)
                .values({ userId, ...values })
                .onDuplicateKeyUpdate({ set: values })
        },
    }

    return { credentialDb, usageDb, resultDb, settingsDb }
}

export const composeAi = (db: Database) => {
    const now = () => Date.now()
    const getProvider = getAiProvider
    const dbs = createAiDbs(db)

    const credentialService = createAiCredentialService({
        getProvider,
        getSecretBox: () => loadSecretBox(getEnv()),
        now,
        db: dbs.credentialDb,
    })

    const usageService = createAiUsageService({
        now,
        newRunId: () => newId(RUN_ID_PREFIX),
        db: dbs.usageDb,
    })

    const resultService = createAiResultService({
        newId,
        now,
        db: dbs.resultDb,
    })

    const settingsService = createAiSettingsService({
        now,
        db: dbs.settingsDb,
    })

    const aiFeatureRunner = createAiFeatureRunner({ credentialService, usageService, resultService, settingsService, getProvider, now })

    return {
        aiCredentialService: credentialService,
        aiUsageService: usageService,
        aiResultService: resultService,
        aiSettingsService: settingsService,
        aiFeatureRunner,
    }
}
