import { z } from 'zod'
import { buildProviderJsonSchema, type ProviderJsonSchema } from '@server/service/shared/ai-provider/json-schema'
import { callProvider, createFailureDetail, toHttpFailureDetail, tryParseJson } from '@server/service/shared/ai-provider/http'
import { finalizeStructuredOutput, toFailedGenerate, toSchemaUnsupportedFailure } from '@server/service/shared/ai-provider/result'
import { AI_FAILURE_KIND, aiProviderIdSchema, type AiFailureKind } from '@server/dto/ai'
import { AI_STRICTNESS, type AiProviderAdapter, type AiProviderCallInput } from '@server/service/shared/ai-provider/types'

const OLLAMA_BASE_URL = 'https://ollama.com'
const OLLAMA_DONE_REASON_STOP = 'stop'
const OLLAMA_THINK_BLOCK_PATTERN = /<think>[\s\S]*?<\/think>/g
const OLLAMA_CODE_FENCE_PATTERN = /```(?:json)?\s*([\s\S]*?)```/gi
const OLLAMA_JSON_INSTRUCTION =
    'Respond with exactly one JSON object that conforms to the JSON Schema below. Output only that JSON object, with no markdown fence and no commentary. Every property in the schema is required.'

const ollamaChatResponseSchema = z.object({
    message: z.object({ content: z.string().nullish() }),
    done: z.boolean(),
    done_reason: z.string().nullish(),
    prompt_eval_count: z.number().nullish(),
    prompt_eval_cached_count: z.number().nullish(),
    eval_count: z.number().nullish(),
})

const ollamaModelListSchema = z.object({ models: z.array(z.object({ name: z.string() })) })

const buildHeaders = (apiKey: string) => ({ authorization: `Bearer ${apiKey}` })

const describeFailure = (response: Parameters<typeof toHttpFailureDetail>[0]['response']) =>
    toHttpFailureDetail({ response, quotaSignal: false, providerErrorType: null })

const requestGet = ({ apiKey, signal }: AiProviderCallInput, path: string) =>
    callProvider({
        url: `${OLLAMA_BASE_URL}${path}`,
        method: 'GET',
        headers: buildHeaders(apiKey),
        signal,
        requestIdHeader: null,
        describeFailure,
    })

const buildSystemPrompt = (system: string, jsonSchema: ProviderJsonSchema) =>
    `${system}\n\n${OLLAMA_JSON_INSTRUCTION}\n\n${JSON.stringify(jsonSchema)}`

const listTopLevelObjects = (text: string) => {
    const objects: string[] = []
    let depth = 0
    let start = -1
    let isInString = false
    let isEscaped = false
    for (let index = 0; index < text.length; index += 1) {
        const character = text[index]
        if (isInString) {
            if (isEscaped) isEscaped = false
            else if (character === '\\') isEscaped = true
            else if (character === '"') isInString = false
            continue
        }
        if (character === '"') isInString = depth > 0
        else if (character === '{') {
            if (depth === 0) start = index
            depth += 1
        } else if (character === '}' && depth > 0) {
            depth -= 1
            if (depth === 0) objects.push(text.slice(start, index + 1))
        }
    }
    return objects
}

/**
 * Best-effort extraction for models that cannot constrain their output: tries the whole text, then fenced
 * code blocks, then every top-level `{...}` span, after removing `<think>` blocks.
 */
export const parseLenientJson = (text: string): ReturnType<typeof tryParseJson> => {
    const visibleText = text.replace(OLLAMA_THINK_BLOCK_PATTERN, '').trim()
    const direct = tryParseJson(visibleText)
    if (direct.ok) return direct
    const candidates = [
        ...Array.from(visibleText.matchAll(OLLAMA_CODE_FENCE_PATTERN), (match) => match[1].trim()),
        ...listTopLevelObjects(visibleText),
    ]
    return candidates.map(tryParseJson).find((parsed) => parsed.ok) ?? { ok: false }
}

export const ollamaAdapter = {
    id: aiProviderIdSchema.enum.ollama,
    strictness: AI_STRICTNESS.BEST_EFFORT,

    validateKey: async (input) => {
        const result = await requestGet(input, '/api/balance')
        return result.ok ? { ok: true } : { ok: false, ...result.detail }
    },

    listModels: async (input) => {
        const result = await requestGet(input, '/api/tags')
        if (!result.ok) return { ok: false, ...result.detail }
        const parsed = ollamaModelListSchema.safeParse(result.response.body)
        if (!parsed.success) return { ok: false, ...createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { httpStatus: result.response.status }) }
        return { ok: true, models: parsed.data.models.map((model) => ({ id: model.name, label: model.name, structured: null })) }
    },

    generateStructured: async (input) => {
        const schema = buildProviderJsonSchema(input.schema)
        if (!schema.ok) return toSchemaUnsupportedFailure()

        const result = await callProvider({
            url: `${OLLAMA_BASE_URL}/api/chat`,
            method: 'POST',
            headers: buildHeaders(input.apiKey),
            body: {
                model: input.model,
                stream: false,
                messages: [
                    { role: 'system', content: buildSystemPrompt(input.system, schema.jsonSchema) },
                    { role: 'user', content: input.input },
                ],
                options: { num_predict: input.maxOutputTokens },
            },
            signal: input.signal,
            requestIdHeader: null,
            describeFailure,
        })
        if (!result.ok) return toFailedGenerate(result.detail)

        const httpStatus = result.response.status
        const parsed = ollamaChatResponseSchema.safeParse(result.response.body)
        if (!parsed.success) return toFailedGenerate(createFailureDetail(AI_FAILURE_KIND.INVALID_OUTPUT, { httpStatus }))

        const {
            message,
            done,
            done_reason: doneReason,
            prompt_eval_count: promptTokens,
            prompt_eval_cached_count: cachedTokens,
            eval_count: outputTokens,
        } = parsed.data
        const usage = { inputTokens: promptTokens ?? 0, outputTokens: outputTokens ?? 0, cachedInputTokens: cachedTokens ?? 0 }
        const toFailure = (kind: AiFailureKind) => toFailedGenerate(createFailureDetail(kind, { httpStatus }), usage)

        if (!done) return toFailure(AI_FAILURE_KIND.INVALID_OUTPUT)
        if (doneReason != null && doneReason !== OLLAMA_DONE_REASON_STOP) return toFailure(AI_FAILURE_KIND.TRUNCATED)

        return finalizeStructuredOutput({
            schema: input.schema,
            text: message.content ?? '',
            parseText: parseLenientJson,
            usage,
            requestId: null,
            httpStatus,
        })
    },
} satisfies AiProviderAdapter
