import { z } from 'zod'
import { buildProviderJsonSchema } from '@server/service/shared/ai-provider/json-schema'
import {
    callProvider,
    createFailureDetail,
    HTTP_STATUS,
    toHttpFailureDetail,
    type ProviderHttpResponse,
} from '@server/service/shared/ai-provider/http'
import { finalizeStructuredOutput, toFailedGenerate, toSchemaUnsupportedFailure } from '@server/service/shared/ai-provider/result'
import { AI_FAILURE_KIND, aiProviderIdSchema, type AiFailureKind } from '@server/dto/ai'
import { AI_STRICTNESS, type AiProviderAdapter, type AiProviderCallInput } from '@server/service/shared/ai-provider/types'

const ANTHROPIC_BASE_URL = 'https://api.anthropic.com'
const ANTHROPIC_VERSION = '2023-06-01'
const ANTHROPIC_REQUEST_ID_HEADER = 'request-id'
const ANTHROPIC_MODELS_PAGE_LIMIT = 1000
const ANTHROPIC_KEY_CHECK_PAGE_LIMIT = 1
const ANTHROPIC_SPEND_LIMIT_ERROR_CODE = 'enforced_spend_limit_reached'
const ANTHROPIC_QUOTA_MESSAGE_PATTERN = /credit balance|spend limit|usage limit/i
const ANTHROPIC_STOP_REASON = {
    END_TURN: 'end_turn',
    STOP_SEQUENCE: 'stop_sequence',
    MAX_TOKENS: 'max_tokens',
    CONTEXT_WINDOW_EXCEEDED: 'model_context_window_exceeded',
    REFUSAL: 'refusal',
} as const
const ANTHROPIC_TEXT_BLOCK_TYPE = 'text'

const anthropicErrorBodySchema = z.object({
    error: z.object({
        type: z.string().nullish(),
        message: z.string().nullish(),
        details: z.object({ error_code: z.string().nullish() }).nullish(),
    }),
})

const anthropicMessageSchema = z.object({
    stop_reason: z.string().nullish(),
    content: z.array(z.object({ type: z.string(), text: z.string().nullish() })),
    usage: z
        .object({
            input_tokens: z.number().nullish(),
            output_tokens: z.number().nullish(),
            cache_read_input_tokens: z.number().nullish(),
            cache_creation_input_tokens: z.number().nullish(),
        })
        .nullish(),
})

const anthropicModelListSchema = z.object({
    data: z.array(
        z.object({
            id: z.string(),
            display_name: z.string().nullish(),
            capabilities: z.object({ structured_outputs: z.object({ supported: z.boolean() }).nullish() }).nullish(),
        }),
    ),
})

const buildHeaders = (apiKey: string) => ({ 'x-api-key': apiKey, 'anthropic-version': ANTHROPIC_VERSION })

const describeFailure = (response: ProviderHttpResponse) => {
    const parsed = anthropicErrorBodySchema.safeParse(response.body)
    const error = parsed.success ? parsed.data.error : null
    const quotaSignal =
        error?.details?.error_code === ANTHROPIC_SPEND_LIMIT_ERROR_CODE ||
        (response.status === HTTP_STATUS.BAD_REQUEST && ANTHROPIC_QUOTA_MESSAGE_PATTERN.test(error?.message ?? ''))
    return toHttpFailureDetail({ response, quotaSignal, providerErrorType: error?.type })
}

const requestModelList = ({ apiKey, signal }: AiProviderCallInput, limit: number) =>
    callProvider({
        url: `${ANTHROPIC_BASE_URL}/v1/models?limit=${limit}`,
        method: 'GET',
        headers: buildHeaders(apiKey),
        signal,
        requestIdHeader: ANTHROPIC_REQUEST_ID_HEADER,
        describeFailure,
    })

export const anthropicAdapter = {
    id: aiProviderIdSchema.enum.anthropic,
    strictness: AI_STRICTNESS.NATIVE,

    validateKey: async (input) => {
        const result = await requestModelList(input, ANTHROPIC_KEY_CHECK_PAGE_LIMIT)
        return result.ok ? { ok: true } : { ok: false, ...result.detail }
    },

    listModels: async (input) => {
        const result = await requestModelList(input, ANTHROPIC_MODELS_PAGE_LIMIT)
        if (!result.ok) return { ok: false, ...result.detail }
        const parsed = anthropicModelListSchema.safeParse(result.response.body)
        if (!parsed.success) {
            return {
                ok: false,
                ...createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { requestId: result.response.requestId, httpStatus: result.response.status }),
            }
        }
        return {
            ok: true,
            models: parsed.data.data
                .map((model) => ({
                    id: model.id,
                    label: model.display_name ?? model.id,
                    structured: model.capabilities?.structured_outputs?.supported ?? null,
                }))
                .filter((model) => model.structured !== false),
        }
    },

    generateStructured: async (input) => {
        const schema = buildProviderJsonSchema(input.schema)
        if (!schema.ok) return toSchemaUnsupportedFailure()

        const result = await callProvider({
            url: `${ANTHROPIC_BASE_URL}/v1/messages`,
            method: 'POST',
            headers: buildHeaders(input.apiKey),
            body: {
                model: input.model,
                max_tokens: input.maxOutputTokens,
                system: input.system,
                messages: [{ role: 'user', content: input.input }],
                output_config: { format: { type: 'json_schema', schema: schema.jsonSchema } },
            },
            signal: input.signal,
            requestIdHeader: ANTHROPIC_REQUEST_ID_HEADER,
            describeFailure,
        })
        if (!result.ok) return toFailedGenerate(result.detail)

        const { requestId, status } = result.response
        const parsed = anthropicMessageSchema.safeParse(result.response.body)
        if (!parsed.success) return toFailedGenerate(createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { requestId, httpStatus: status }))

        const { stop_reason: stopReason, content, usage: rawUsage } = parsed.data
        const cachedInputTokens = rawUsage?.cache_read_input_tokens ?? 0
        const usage = {
            inputTokens: (rawUsage?.input_tokens ?? 0) + (rawUsage?.cache_creation_input_tokens ?? 0) + cachedInputTokens,
            outputTokens: rawUsage?.output_tokens ?? 0,
            cachedInputTokens,
        }
        const toFailure = (kind: AiFailureKind) => toFailedGenerate(createFailureDetail(kind, { requestId, httpStatus: status }), usage)

        if (stopReason === ANTHROPIC_STOP_REASON.REFUSAL) return toFailure(AI_FAILURE_KIND.REFUSED)
        if (stopReason === ANTHROPIC_STOP_REASON.MAX_TOKENS || stopReason === ANTHROPIC_STOP_REASON.CONTEXT_WINDOW_EXCEEDED) {
            return toFailure(AI_FAILURE_KIND.TRUNCATED)
        }
        if (stopReason !== ANTHROPIC_STOP_REASON.END_TURN && stopReason !== ANTHROPIC_STOP_REASON.STOP_SEQUENCE)
            return toFailure(AI_FAILURE_KIND.INVALID_OUTPUT)

        return finalizeStructuredOutput({
            schema: input.schema,
            text: content
                .filter((block) => block.type === ANTHROPIC_TEXT_BLOCK_TYPE)
                .map((block) => block.text ?? '')
                .join(''),
            usage,
            requestId,
            httpStatus: status,
        })
    },
} satisfies AiProviderAdapter
