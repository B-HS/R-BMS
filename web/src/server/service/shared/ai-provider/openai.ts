import { z } from 'zod'
import { buildProviderJsonSchema } from '@server/service/shared/ai-provider/json-schema'
import { callProvider, createFailureDetail, toHttpFailureDetail, type ProviderHttpResponse } from '@server/service/shared/ai-provider/http'
import { finalizeStructuredOutput, toFailedGenerate, toSchemaUnsupportedFailure } from '@server/service/shared/ai-provider/result'
import { AI_FAILURE_KIND, aiProviderIdSchema, type AiFailureKind } from '@server/dto/ai'
import { AI_STRICTNESS, type AiProviderAdapter, type AiProviderCallInput } from '@server/service/shared/ai-provider/types'

const OPENAI_BASE_URL = 'https://api.openai.com'
const OPENAI_REQUEST_ID_HEADER = 'x-request-id'
const OPENAI_SCHEMA_NAME_MAX_LENGTH = 64
const OPENAI_SCHEMA_NAME_INVALID_CHARACTERS = /[^A-Za-z0-9_-]/g
const OPENAI_QUOTA_ERROR_CODES = new Set([
    'insufficient_quota',
    'credit_balance_exhausted',
    'organization_spend_limit_exceeded',
    'project_spend_limit_exceeded',
    'organization_usage_limit_exceeded',
])
const OPENAI_RESPONSE_STATUS = {
    COMPLETED: 'completed',
    INCOMPLETE: 'incomplete',
    FAILED: 'failed',
} as const
const OPENAI_INCOMPLETE_REASON_CONTENT_FILTER = 'content_filter'
const OPENAI_FAILED_CODE = {
    RATE_LIMIT_EXCEEDED: 'rate_limit_exceeded',
    INVALID_PROMPT: 'invalid_prompt',
} as const
const OPENAI_OUTPUT_TEXT_TYPE = 'output_text'
const OPENAI_REFUSAL_TYPE = 'refusal'
const OPENAI_MESSAGE_TYPE = 'message'

const openaiErrorBodySchema = z.object({
    error: z.object({
        type: z.string().nullish(),
        code: z.string().nullish(),
    }),
})

const openaiResponseSchema = z.object({
    status: z.string(),
    error: z.object({ code: z.string().nullish() }).nullish(),
    incomplete_details: z.object({ reason: z.string().nullish() }).nullish(),
    output: z
        .array(
            z.object({
                type: z.string(),
                content: z.array(z.object({ type: z.string(), text: z.string().nullish() })).nullish(),
            }),
        )
        .default([]),
    usage: z
        .object({
            input_tokens: z.number().nullish(),
            output_tokens: z.number().nullish(),
            input_tokens_details: z.object({ cached_tokens: z.number().nullish() }).nullish(),
        })
        .nullish(),
})

const openaiModelListSchema = z.object({ data: z.array(z.object({ id: z.string() })) })

const buildHeaders = (apiKey: string) => ({ authorization: `Bearer ${apiKey}` })

const describeFailure = (response: ProviderHttpResponse) => {
    const parsed = openaiErrorBodySchema.safeParse(response.body)
    const error = parsed.success ? parsed.data.error : null
    const quotaSignal = [error?.code, error?.type].some((value) => value != null && OPENAI_QUOTA_ERROR_CODES.has(value))
    return toHttpFailureDetail({ response, quotaSignal, providerErrorType: error?.code ?? error?.type })
}

const requestModelList = ({ apiKey, signal }: AiProviderCallInput) =>
    callProvider({
        url: `${OPENAI_BASE_URL}/v1/models`,
        method: 'GET',
        headers: buildHeaders(apiKey),
        signal,
        requestIdHeader: OPENAI_REQUEST_ID_HEADER,
        describeFailure,
    })

const resolveFailedKind = (code: string | null | undefined): AiFailureKind => {
    if (code === OPENAI_FAILED_CODE.RATE_LIMIT_EXCEEDED) return AI_FAILURE_KIND.RATE_LIMITED
    if (code === OPENAI_FAILED_CODE.INVALID_PROMPT) return AI_FAILURE_KIND.REFUSED
    return AI_FAILURE_KIND.UNAVAILABLE
}

export const openaiAdapter = {
    id: aiProviderIdSchema.enum.openai,
    strictness: AI_STRICTNESS.NATIVE,

    validateKey: async (input) => {
        const result = await requestModelList(input)
        return result.ok ? { ok: true } : { ok: false, ...result.detail }
    },

    listModels: async (input) => {
        const result = await requestModelList(input)
        if (!result.ok) return { ok: false, ...result.detail }
        const parsed = openaiModelListSchema.safeParse(result.response.body)
        if (!parsed.success) {
            return {
                ok: false,
                ...createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { requestId: result.response.requestId, httpStatus: result.response.status }),
            }
        }
        return { ok: true, models: parsed.data.data.map((model) => ({ id: model.id, label: model.id, structured: null })) }
    },

    generateStructured: async (input) => {
        const schema = buildProviderJsonSchema(input.schema)
        if (!schema.ok) return toSchemaUnsupportedFailure()

        const result = await callProvider({
            url: `${OPENAI_BASE_URL}/v1/responses`,
            method: 'POST',
            headers: buildHeaders(input.apiKey),
            body: {
                model: input.model,
                instructions: input.system,
                input: input.input,
                max_output_tokens: input.maxOutputTokens,
                store: false,
                text: {
                    format: {
                        type: 'json_schema',
                        name: input.schemaName.replace(OPENAI_SCHEMA_NAME_INVALID_CHARACTERS, '_').slice(0, OPENAI_SCHEMA_NAME_MAX_LENGTH),
                        strict: true,
                        schema: schema.jsonSchema,
                    },
                },
            },
            signal: input.signal,
            requestIdHeader: OPENAI_REQUEST_ID_HEADER,
            describeFailure,
        })
        if (!result.ok) return toFailedGenerate(result.detail)

        const { requestId, status: httpStatus } = result.response
        const parsed = openaiResponseSchema.safeParse(result.response.body)
        if (!parsed.success) return toFailedGenerate(createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { requestId, httpStatus }))

        const { status, error, incomplete_details: incompleteDetails, output, usage: rawUsage } = parsed.data
        const usage = {
            inputTokens: rawUsage?.input_tokens ?? 0,
            outputTokens: rawUsage?.output_tokens ?? 0,
            cachedInputTokens: rawUsage?.input_tokens_details?.cached_tokens ?? 0,
        }
        const toFailure = (kind: AiFailureKind) => toFailedGenerate(createFailureDetail(kind, { requestId, httpStatus }), usage)

        if (status === OPENAI_RESPONSE_STATUS.FAILED) return toFailure(resolveFailedKind(error?.code))
        if (status === OPENAI_RESPONSE_STATUS.INCOMPLETE) {
            return toFailure(
                incompleteDetails?.reason === OPENAI_INCOMPLETE_REASON_CONTENT_FILTER ? AI_FAILURE_KIND.REFUSED : AI_FAILURE_KIND.TRUNCATED,
            )
        }
        if (status !== OPENAI_RESPONSE_STATUS.COMPLETED) return toFailure(AI_FAILURE_KIND.UNAVAILABLE)

        const contents = output.filter((item) => item.type === OPENAI_MESSAGE_TYPE).flatMap((item) => item.content ?? [])
        if (contents.some((content) => content.type === OPENAI_REFUSAL_TYPE)) return toFailure(AI_FAILURE_KIND.REFUSED)

        return finalizeStructuredOutput({
            schema: input.schema,
            text: contents
                .filter((content) => content.type === OPENAI_OUTPUT_TEXT_TYPE)
                .map((content) => content.text ?? '')
                .join(''),
            usage,
            requestId,
            httpStatus,
        })
    },
} satisfies AiProviderAdapter
