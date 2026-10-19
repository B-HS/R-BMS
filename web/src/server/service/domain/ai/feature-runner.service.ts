import type { z } from 'zod'
import { summarizeError } from '@server/lib/error-summary'
import type { AiProviderResolver } from '@server/service/domain/ai/ai.type'
import type { AiCredentialService } from '@server/service/domain/ai/credential.service'
import { hashAiInput, type AiResultService } from '@server/service/domain/ai/result.service'
import type { AiSettingsService } from '@server/service/domain/ai/settings.service'
import type { AiUsageService } from '@server/service/domain/ai/usage.service'
import { AI_FAILURE_KIND, type AiEffort, type AiProviderId, type AiUsage } from '@server/dto/ai'

/** How long one provider call may run before it is aborted. Shorter than the AI routes' `maxDuration` and the slot hold. */
export const AI_CALL_TIMEOUT_MS = 240_000

export type AiFeatureRunInput<TSchema extends z.ZodType> = {
    userId: string
    feature: string
    subjectKey: string
    analysisVersion: number
    promptVersion: number
    digest: unknown
    system: string
    schemaName: string
    schema: TSchema
    maxOutputTokens: number
    effort?: AiEffort
    /** Feature checks after the schema (for example that every `key` exists in the digest); return null to reject the output. */
    refine?: (data: z.infer<TSchema>) => z.infer<TSchema> | null
}

export type AiFeatureRunOutput<TData> = {
    output: TData
    cached: boolean
    provider: AiProviderId
    model: string
    usage: AiUsage
    createdAt: number
}

type AiRunHashFields = Pick<
    AiFeatureRunInput<z.ZodType>,
    'digest' | 'system' | 'schemaName' | 'analysisVersion' | 'promptVersion' | 'maxOutputTokens' | 'effort'
>

/**
 * The cache key of one run: everything that changes the provider's answer. A stored result whose `inputHash` equals this for its
 * own provider and model was made from the same digest, prompt and schema, so a screen can tell a current result from a stale one.
 */
export const hashAiRunInput = (input: AiRunHashFields, target: { provider: AiProviderId; model: string }) =>
    hashAiInput({
        digest: input.digest,
        system: input.system,
        schemaName: input.schemaName,
        analysisVersion: input.analysisVersion,
        promptVersion: input.promptVersion,
        maxOutputTokens: input.maxOutputTokens,
        effort: input.effort ?? null,
        provider: target.provider,
        model: target.model,
    })

type AiFeatureRunnerDeps = {
    credentialService: Pick<AiCredentialService, 'list' | 'withApiKey' | 'markRejected'>
    usageService: Pick<AiUsageService, 'reserve' | 'record' | 'release'>
    resultService: Pick<AiResultService, 'findCached' | 'save'>
    settingsService: Pick<AiSettingsService, 'getUserSetting' | 'resolveLimits'>
    getProvider: AiProviderResolver
    now: () => number
}

export const createAiFeatureRunner = (deps: AiFeatureRunnerDeps) => {
    const chooseCredential = async (userId: string) => {
        const [credentials, setting] = await Promise.all([deps.credentialService.list(userId), deps.settingsService.getUserSetting(userId)])
        const preferred = credentials.find((credential) => credential.provider === setting.default_provider)
        return preferred ?? credentials.toSorted((left, right) => left.created_at - right.created_at)[0] ?? null
    }

    /** Stores the result and returns its time. A failed store is logged without the output and the run still answers. */
    const saveResult = async (params: Parameters<AiFeatureRunnerDeps['resultService']['save']>[0]) => {
        try {
            return (await deps.resultService.save(params)).createdAt
        } catch (error) {
            console.error('AI result save failed', { feature: params.feature, ...summarizeError(error) })
            return deps.now()
        }
    }

    /**
     * Runs one AI feature call end to end: pick the user's provider and model, return the cached result for the same input, check
     * the limits and take a concurrency slot, decrypt the key, call the provider with no database connection held, re-validate the
     * output with the feature's Zod schema, record the token usage, store the result, then give the slot back. Failures are not
     * cached and there is no automatic retry. Giving the slot back and storing the result are best effort once the provider has
     * answered: a lost slot expires on its own and a lost result is only a missed cache entry.
     */
    const run = async <TSchema extends z.ZodType>(input: AiFeatureRunInput<TSchema>) => {
        const { userId } = input
        const credential = await chooseCredential(userId)
        if (!credential) return { ok: false, reason: 'credential_not_found' } as const
        const { provider, model } = credential
        if (!model) return { ok: false, reason: 'model_not_selected', provider } as const

        const inputHash = hashAiRunInput(input, { provider, model })
        const resultKey = { userId, feature: input.feature, subjectKey: input.subjectKey }
        const cached = await deps.resultService.findCached({ ...resultKey, inputHash })
        const cachedOutput = cached ? input.schema.safeParse(cached.output) : null
        if (cached && cachedOutput?.success) {
            const value = {
                output: cachedOutput.data,
                cached: true,
                provider: cached.provider,
                model: cached.model,
                usage: cached.usage,
                createdAt: cached.createdAt,
            } satisfies AiFeatureRunOutput<z.infer<TSchema>>
            return { ok: true, value } as const
        }

        const limits = await deps.settingsService.resolveLimits(userId)
        const called = await deps.credentialService.withApiKey({ userId, provider }, async (apiKey) => {
            const reserved = await deps.usageService.reserve({ userId, limits })
            if (!reserved.ok) return { status: 'limited' as const, limit: reserved.limit }
            try {
                const generated = await deps.getProvider(provider).generateStructured({
                    apiKey,
                    model,
                    system: input.system,
                    input: JSON.stringify(input.digest),
                    schemaName: input.schemaName,
                    schema: input.schema,
                    maxOutputTokens: input.maxOutputTokens,
                    effort: input.effort,
                    signal: AbortSignal.timeout(AI_CALL_TIMEOUT_MS),
                })
                if (generated.usage) await deps.usageService.record({ hold: reserved.hold, usage: generated.usage })
                return { status: 'called' as const, generated }
            } finally {
                await Promise.allSettled([deps.usageService.release(reserved.hold)])
            }
        })
        if (!called.ok) return called
        if (called.value.status === 'limited') return { ok: false, reason: 'usage_limit', limit: called.value.limit } as const

        const { generated } = called.value
        if (!generated.ok) {
            if (generated.kind === AI_FAILURE_KIND.INVALID_KEY) await deps.credentialService.markRejected({ userId, provider })
            return { ok: false, reason: 'provider', provider, detail: generated } as const
        }
        const validated = input.schema.safeParse(generated.data)
        if (!validated.success) return { ok: false, reason: 'output_invalid', provider } as const
        const refined = input.refine ? input.refine(validated.data) : validated.data
        if (refined === null) return { ok: false, reason: 'output_invalid', provider } as const

        const createdAt = await saveResult({
            ...resultKey,
            inputHash,
            analysisVersion: input.analysisVersion,
            promptVersion: input.promptVersion,
            provider,
            model,
            output: refined,
            usage: generated.usage,
        })
        const value = { output: refined, cached: false, provider, model, usage: generated.usage, createdAt } satisfies AiFeatureRunOutput<
            z.infer<TSchema>
        >
        return { ok: true, value } as const
    }

    return { run }
}

export type AiFeatureRunner = ReturnType<typeof createAiFeatureRunner>
