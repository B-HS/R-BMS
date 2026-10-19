import { describe, expect, test } from 'bun:test'
import { http, HttpResponse, type JsonBodyType } from 'msw'
import { anthropicAdapter } from '@server/service/shared/ai-provider/anthropic'
import { listJsonSchemaViolations } from '@server/service/shared/ai-provider/json-schema'
import { AI_FAILURE_KIND } from '@server/dto/ai'
import { AI_STRICTNESS } from '@server/service/shared/ai-provider/types'
import {
    abortedCallInput,
    callInput,
    FAKE_API_KEY,
    findLeakedTexts,
    generateInput,
    MAX_OUTPUT_TOKENS,
    MODEL_ID,
    OVERSIZED_REPORT,
    readJsonBody,
    REQUEST_ID,
    RETRY_AFTER_SECONDS,
    server,
    SYSTEM_PROMPT,
    setupMswServer,
    USER_INPUT,
    VALID_REPORT,
    watchConsole,
} from './support'

const MESSAGES_URL = 'https://api.anthropic.com/v1/messages'
const MODELS_URL = 'https://api.anthropic.com/v1/models'
const REQUEST_ID_HEADERS = { 'request-id': REQUEST_ID }

const messageBody = (overrides: Record<string, unknown> = {}) => ({
    id: 'msg_1',
    type: 'message',
    role: 'assistant',
    model: MODEL_ID,
    content: [{ type: 'text', text: JSON.stringify(VALID_REPORT) }],
    stop_reason: 'end_turn',
    usage: { input_tokens: 100, output_tokens: 50, cache_read_input_tokens: 20, cache_creation_input_tokens: 10 },
    ...overrides,
})

const replyMessage = (body: JsonBodyType, status = 200, headers: Record<string, string> = {}) =>
    http.post(MESSAGES_URL, () => HttpResponse.json(body, { status, headers: { ...REQUEST_ID_HEADERS, ...headers } }))

const replyError = (status: number, error: Record<string, unknown>, headers: Record<string, string> = {}) =>
    replyMessage({ type: 'error', error, request_id: REQUEST_ID }, status, headers)

setupMswServer()

describe('anthropicAdapter 메타', () => {
    test('공급자 id 와 형식 보장 등급이 계약과 같다', () => {
        expect(anthropicAdapter.id).toBe('anthropic')
        expect(anthropicAdapter.strictness).toBe(AI_STRICTNESS.NATIVE)
    })
})

describe('anthropicAdapter.generateStructured 성공', () => {
    test('구조화 출력 요청을 보내고 검증된 데이터와 정규화한 사용량을 돌려준다', async () => {
        const requests: { headers: Headers; body: Record<string, unknown>; redirect: RequestRedirect }[] = []
        server.use(
            http.post(MESSAGES_URL, async ({ request }) => {
                requests.push({ headers: request.headers, body: await readJsonBody(request), redirect: request.redirect })
                return HttpResponse.json(messageBody(), { headers: REQUEST_ID_HEADERS })
            }),
        )

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toEqual({
            ok: true,
            data: VALID_REPORT,
            usage: { inputTokens: 130, outputTokens: 50, cachedInputTokens: 20 },
            requestId: REQUEST_ID,
        })
        expect(requests[0].headers.get('x-api-key')).toBe(FAKE_API_KEY)
        expect(requests[0].headers.get('anthropic-version')).toBe('2023-06-01')
        expect(requests[0].redirect).toBe('error')
        expect(requests[0].body).toMatchObject({
            model: MODEL_ID,
            max_tokens: MAX_OUTPUT_TOKENS,
            system: SYSTEM_PROMPT,
            messages: [{ role: 'user', content: USER_INPUT }],
        })
        expect(requests[0].body).not.toHaveProperty('thinking')
        expect(requests[0].body).not.toHaveProperty('tool_choice')
        expect(requests[0].body).not.toHaveProperty('temperature')
        const outputConfig = requests[0].body.output_config as { format: { type: string; schema: unknown }; effort?: string }
        expect(outputConfig.format.type).toBe('json_schema')
        expect(outputConfig).not.toHaveProperty('effort')
        expect(listJsonSchemaViolations(outputConfig.format.schema)).toEqual([])
        expect(JSON.stringify(outputConfig.format.schema)).not.toContain('maxLength')
    })

    test('effort 를 받아도 모델이 지원하는지 알 수 없으므로 보내지 않는다', async () => {
        const outputConfigs: unknown[] = []
        server.use(
            http.post(MESSAGES_URL, async ({ request }) => {
                outputConfigs.push(((await request.json()) as { output_config: unknown }).output_config)
                return HttpResponse.json(messageBody(), { headers: REQUEST_ID_HEADERS })
            }),
        )

        expect((await anthropicAdapter.generateStructured({ ...generateInput(), effort: 'low' })).ok).toBe(true)

        expect(outputConfigs).toHaveLength(1)
        expect(outputConfigs[0]).not.toHaveProperty('effort')
    })

    test('thinking 블록과 여러 text 블록이 섞여도 text 만 이어 붙여 읽는다', async () => {
        const json = JSON.stringify(VALID_REPORT)
        server.use(
            replyMessage(
                messageBody({
                    content: [
                        { type: 'thinking', thinking: '', signature: 'sig' },
                        { type: 'text', text: json.slice(0, 10) },
                        { type: 'text', text: json.slice(10) },
                    ],
                }),
            ),
        )

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: true, data: VALID_REPORT })
    })

    test('usage 필드가 없으면 0 으로 센다', async () => {
        server.use(replyMessage(messageBody({ usage: undefined })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: true, usage: { inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 } })
    })
})

describe('anthropicAdapter.generateStructured 응답 실패', () => {
    test('stop_reason refusal 은 refused 이고 사용량과 요청 id 를 남긴다', async () => {
        server.use(replyMessage(messageBody({ stop_reason: 'refusal', content: [] })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.REFUSED,
            requestId: REQUEST_ID,
            httpStatus: 200,
            usage: { inputTokens: 130, outputTokens: 50, cachedInputTokens: 20 },
        })
    })

    test.each(['max_tokens', 'model_context_window_exceeded'])('stop_reason %s 는 truncated 다', async (stopReason) => {
        server.use(replyMessage(messageBody({ stop_reason: stopReason, content: [{ type: 'text', text: '{"summary":"cut' }] })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.TRUNCATED, requestId: REQUEST_ID })
    })

    test.each(['tool_use', 'pause_turn', null])('예상 밖 stop_reason %p 는 invalid_output 이다', async (stopReason) => {
        server.use(replyMessage(messageBody({ stop_reason: stopReason })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('JSON 이 아닌 본문은 invalid_output 이다', async () => {
        server.use(replyMessage(messageBody({ content: [{ type: 'text', text: 'Here is your report.' }] })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT, usage: { outputTokens: 50 } })
    })

    test('공급자 스키마에서 뺀 제약도 원래 Zod 로 다시 검증해 거절한다', async () => {
        server.use(replyMessage(messageBody({ content: [{ type: 'text', text: JSON.stringify(OVERSIZED_REPORT) }] })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT, requestId: REQUEST_ID })
    })

    test('스키마 필드가 빠진 JSON 은 invalid_output 이다', async () => {
        server.use(replyMessage(messageBody({ content: [{ type: 'text', text: JSON.stringify({ summary: 'only summary' }) }] })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('2xx 인데 본문이 JSON 이 아니면 invalid_output 이다', async () => {
        server.use(http.post(MESSAGES_URL, () => new HttpResponse('<html>gateway</html>', { status: 200, headers: REQUEST_ID_HEADERS })))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT, requestId: REQUEST_ID, usage: null })
    })

    test('변환할 수 없는 스키마는 요청을 보내지 않고 bad_request 다', async () => {
        const result = await anthropicAdapter.generateStructured({ ...generateInput(), schema: generateInput().schema.nullable() })

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.BAD_REQUEST, providerErrorType: 'schema_unsupported' })
    })
})

describe('anthropicAdapter.generateStructured HTTP 오류', () => {
    test.each([
        [401, { type: 'authentication_error', message: 'invalid x-api-key' }, {}, AI_FAILURE_KIND.INVALID_KEY],
        [403, { type: 'permission_error', message: 'forbidden' }, {}, AI_FAILURE_KIND.FORBIDDEN],
        [402, { type: 'billing_error', message: 'billing' }, {}, AI_FAILURE_KIND.QUOTA],
        [429, { type: 'rate_limit_error', message: 'slow down' }, { 'retry-after': String(RETRY_AFTER_SECONDS) }, AI_FAILURE_KIND.RATE_LIMITED],
        [429, { type: 'rate_limit_error', message: 'cap', details: { error_code: 'enforced_spend_limit_reached' } }, {}, AI_FAILURE_KIND.QUOTA],
        [400, { type: 'invalid_request_error', message: 'Your credit balance is too low to access the Anthropic API.' }, {}, AI_FAILURE_KIND.QUOTA],
        [400, { type: 'invalid_request_error', message: 'Schema is too complex for compilation.' }, {}, AI_FAILURE_KIND.BAD_REQUEST],
        [404, { type: 'not_found_error', message: 'model: nope' }, {}, AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [413, { type: 'request_too_large', message: 'too large' }, {}, AI_FAILURE_KIND.BAD_REQUEST],
        [500, { type: 'api_error', message: 'oops' }, {}, AI_FAILURE_KIND.UNAVAILABLE],
        [504, { type: 'timeout_error', message: 'timeout' }, {}, AI_FAILURE_KIND.UNAVAILABLE],
        [529, { type: 'overloaded_error', message: 'busy' }, {}, AI_FAILURE_KIND.UNAVAILABLE],
    ])('HTTP %d %o 는 %s 로 분류한다', async (status, error, headers, kind) => {
        server.use(replyError(status, error, headers))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind, httpStatus: status, requestId: REQUEST_ID, usage: null })
    })

    test('429 의 retry-after 는 초 단위로 읽기만 하고 재시도하지 않는다', async () => {
        let callCount = 0
        server.use(
            http.post(MESSAGES_URL, () => {
                callCount += 1
                return HttpResponse.json(
                    { type: 'error', error: { type: 'rate_limit_error', message: 'slow' } },
                    { status: 429, headers: { ...REQUEST_ID_HEADERS, 'retry-after': String(RETRY_AFTER_SECONDS) } },
                )
            }),
        )

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.RATE_LIMITED,
            retryAfterSeconds: RETRY_AFTER_SECONDS,
            providerErrorType: 'rate_limit_error',
        })
        expect(callCount).toBe(1)
    })

    test('연결 실패는 unavailable 이고 메시지·URL·키를 결과에 남기지 않는다', async () => {
        const consoleSpies = watchConsole()
        server.use(http.post(MESSAGES_URL, () => HttpResponse.error()))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toEqual({
            ok: false,
            kind: AI_FAILURE_KIND.UNAVAILABLE,
            requestId: null,
            httpStatus: null,
            providerErrorType: null,
            retryAfterSeconds: null,
            usage: null,
        })
        expect(findLeakedTexts(result, [FAKE_API_KEY, 'api.anthropic.com'])).toEqual([])
        consoleSpies.forEach((spy) => expect(spy).not.toHaveBeenCalled())
    })

    test('이미 중단된 signal 은 unavailable 이다', async () => {
        server.use(replyMessage(messageBody()))

        const result = await anthropicAdapter.generateStructured({ ...generateInput(), signal: abortedCallInput().signal })

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.UNAVAILABLE })
    })

    test('공급자 오류 본문이 키를 되풀이해도 결과·로그에 나오지 않는다', async () => {
        const consoleSpies = watchConsole()
        const echoed = `invalid x-api-key: ${FAKE_API_KEY}`
        server.use(replyError(401, { type: 'authentication_error', message: echoed }))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_KEY })
        expect(findLeakedTexts(result, [FAKE_API_KEY, echoed])).toEqual([])
        consoleSpies.forEach((spy) => expect(spy).not.toHaveBeenCalled())
    })

    test('성공 결과에도 키가 없다', async () => {
        server.use(replyMessage(messageBody()))

        const result = await anthropicAdapter.generateStructured(generateInput())

        expect(findLeakedTexts(result, [FAKE_API_KEY])).toEqual([])
    })
})

describe('anthropicAdapter.validateKey', () => {
    test('GET /v1/models?limit=1 이 200 이면 ok 다', async () => {
        const requests: { limit: string | null; apiKey: string | null; method: string }[] = []
        server.use(
            http.get(MODELS_URL, ({ request }) => {
                requests.push({
                    limit: new URL(request.url).searchParams.get('limit'),
                    apiKey: request.headers.get('x-api-key'),
                    method: request.method,
                })
                return HttpResponse.json({ data: [], has_more: false })
            }),
        )

        expect(await anthropicAdapter.validateKey(callInput())).toEqual({ ok: true })
        expect(requests).toEqual([{ limit: '1', apiKey: FAKE_API_KEY, method: 'GET' }])
    })

    test('401 은 invalid_key 다', async () => {
        server.use(
            http.get(MODELS_URL, () =>
                HttpResponse.json(
                    { type: 'error', error: { type: 'authentication_error', message: 'bad' } },
                    { status: 401, headers: REQUEST_ID_HEADERS },
                ),
            ),
        )

        const result = await anthropicAdapter.validateKey(callInput())

        expect(result).toEqual({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_KEY,
            requestId: REQUEST_ID,
            httpStatus: 401,
            providerErrorType: 'authentication_error',
            retryAfterSeconds: null,
        })
    })

    test('연결 실패는 unavailable 이다', async () => {
        server.use(http.get(MODELS_URL, () => HttpResponse.error()))

        expect(await anthropicAdapter.validateKey(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.UNAVAILABLE })
    })
})

describe('anthropicAdapter.listModels', () => {
    test('모델을 모두 달라고 요청하고 구조화 출력을 지원하지 않는 모델은 뺀다', async () => {
        const limits: (string | null)[] = []
        server.use(
            http.get(MODELS_URL, ({ request }) => {
                limits.push(new URL(request.url).searchParams.get('limit'))
                return HttpResponse.json({
                    data: [
                        { id: 'claude-a', display_name: 'Claude A', capabilities: { structured_outputs: { supported: true } } },
                        { id: 'claude-b', display_name: 'Claude B', capabilities: { structured_outputs: { supported: false } } },
                        { id: 'claude-c', capabilities: null },
                    ],
                    has_more: false,
                })
            }),
        )

        const result = await anthropicAdapter.listModels(callInput())

        expect(limits).toEqual(['1000'])
        expect(result).toEqual({
            ok: true,
            models: [
                { id: 'claude-a', label: 'Claude A', structured: true },
                { id: 'claude-c', label: 'claude-c', structured: null },
            ],
        })
    })

    test('401 은 invalid_key 다', async () => {
        server.use(
            http.get(MODELS_URL, () =>
                HttpResponse.json({ type: 'error', error: { type: 'authentication_error', message: 'bad' } }, { status: 401 }),
            ),
        )

        expect(await anthropicAdapter.listModels(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_KEY })
    })

    test('목록 형식이 틀리면 invalid_output 이다', async () => {
        server.use(http.get(MODELS_URL, () => HttpResponse.json({ models: [] }, { headers: REQUEST_ID_HEADERS })))

        expect(await anthropicAdapter.listModels(callInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_OUTPUT,
            requestId: REQUEST_ID,
        })
    })
})
