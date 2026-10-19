import type { z } from 'zod'
import type { AiEffort, AiFailureKind, AiProviderId, AiUsage } from '@server/dto/ai'

export const AI_STRICTNESS = {
    NATIVE: 'native',
    BEST_EFFORT: 'best_effort',
} as const
export type AiStrictness = (typeof AI_STRICTNESS)[keyof typeof AI_STRICTNESS]

/**
 * Everything the caller may keep about a failed provider call. It never contains
 * the API key, the request, or the provider error body.
 */
export type AiFailureDetail = {
    kind: AiFailureKind
    requestId: string | null
    httpStatus: number | null
    providerErrorType: string | null
    retryAfterSeconds: number | null
}

export type AiModelInfo = {
    id: string
    label: string
    structured: boolean | null
}

export type AiKeyCheckResult = { ok: true } | ({ ok: false } & AiFailureDetail)

export type AiListModelsResult = { ok: true; models: AiModelInfo[] } | ({ ok: false } & AiFailureDetail)

export type AiGenerateResult<TData> =
    { ok: true; data: TData; usage: AiUsage; requestId: string | null } | ({ ok: false; usage: AiUsage | null } & AiFailureDetail)

export type AiProviderCallInput = {
    apiKey: string
    signal: AbortSignal
}

/**
 * `effort` is the depth the feature asks for. No adapter sends it yet: Anthropic answers 400 when a model without effort support
 * (Haiku 4.5, Sonnet 4.5) receives `output_config.effort`, and the stored model choice carries no capability flag to tell them apart.
 * It still enters the result cache key, so turning it on later changes the cache naturally.
 */
export type GenerateStructuredInput<TSchema extends z.ZodType> = AiProviderCallInput & {
    model: string
    system: string
    input: string
    schemaName: string
    schema: TSchema
    maxOutputTokens: number
    effort?: AiEffort
}

export type AiProviderAdapter = {
    id: AiProviderId
    strictness: AiStrictness
    validateKey: (input: AiProviderCallInput) => Promise<AiKeyCheckResult>
    listModels: (input: AiProviderCallInput) => Promise<AiListModelsResult>
    generateStructured: <TSchema extends z.ZodType>(input: GenerateStructuredInput<TSchema>) => Promise<AiGenerateResult<z.infer<TSchema>>>
}
