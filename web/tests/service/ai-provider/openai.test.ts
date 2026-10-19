import { describe, expect, test } from 'bun:test'
import { http, HttpResponse, type JsonBodyType } from 'msw'
import { listJsonSchemaViolations } from '@server/service/shared/ai-provider/json-schema'
import { openaiAdapter } from '@server/service/shared/ai-provider/openai'
import { AI_FAILURE_KIND } from '@server/dto/ai'
import { AI_STRICTNESS } from '@server/service/shared/ai-provider/types'
import {
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
    SCHEMA_NAME,
    server,
    SYSTEM_PROMPT,
    setupMswServer,
    USER_INPUT,
    VALID_REPORT,
    watchConsole,
} from './support'

const RESPONSES_URL = 'https://api.openai.com/v1/responses'
const MODELS_URL = 'https://api.openai.com/v1/models'
const REQUEST_ID_HEADERS = { 'x-request-id': REQUEST_ID }

const messageItem = (content: unknown[]) => ({ type: 'message', role: 'assistant', status: 'completed', content })

const responseBody = (overrides: Record<string, unknown> = {}) => ({
    id: 'resp_1',
    object: 'response',
    status: 'completed',
    error: null,
    incomplete_details: null,
    output: [{ type: 'reasoning', summary: [] }, messageItem([{ type: 'output_text', text: JSON.stringify(VALID_REPORT), annotations: [] }])],
    usage: {
        input_tokens: 120,
        input_tokens_details: { cached_tokens: 64 },
        output_tokens: 40,
        output_tokens_details: { reasoning_tokens: 8 },
        total_tokens: 160,
    },
    ...overrides,
})

const replyResponse = (body: JsonBodyType, status = 200, headers: Record<string, string> = {}) =>
    http.post(RESPONSES_URL, () => HttpResponse.json(body, { status, headers: { ...REQUEST_ID_HEADERS, ...headers } }))

const replyError = (status: number, error: Record<string, unknown>, headers: Record<string, string> = {}) => replyResponse({ error }, status, headers)

setupMswServer()

describe('openaiAdapter 메타', () => {
    test('공급자 id 와 형식 보장 등급이 계약과 같다', () => {
        expect(openaiAdapter.id).toBe('openai')
        expect(openaiAdapter.strictness).toBe(AI_STRICTNESS.NATIVE)
    })
})

describe('openaiAdapter.generateStructured 성공', () => {
    test('Responses API 에 strict json_schema 와 store false 를 보내고 사용량을 정규화한다', async () => {
        const requests: { headers: Headers; body: Record<string, unknown>; redirect: RequestRedirect }[] = []
        server.use(
            http.post(RESPONSES_URL, async ({ request }) => {
                requests.push({ headers: request.headers, body: await readJsonBody(request), redirect: request.redirect })
                return HttpResponse.json(responseBody(), { headers: REQUEST_ID_HEADERS })
            }),
        )

        const result = await openaiAdapter.generateStructured(generateInput())

        expect(result).toEqual({
            ok: true,
            data: VALID_REPORT,
            usage: { inputTokens: 120, outputTokens: 40, cachedInputTokens: 64 },
            requestId: REQUEST_ID,
        })
        expect(requests[0].headers.get('authorization')).toBe(`Bearer ${FAKE_API_KEY}`)
        expect(requests[0].redirect).toBe('error')
        expect(requests[0].body).toMatchObject({
            model: MODEL_ID,
            instructions: SYSTEM_PROMPT,
            input: USER_INPUT,
            max_output_tokens: MAX_OUTPUT_TOKENS,
            store: false,
        })
        const format = (requests[0].body.text as { format: { type: string; name: string; strict: boolean; schema: unknown } }).format
        expect(format).toMatchObject({ type: 'json_schema', name: SCHEMA_NAME, strict: true })
        expect(listJsonSchemaViolations(format.schema)).toEqual([])
        expect(JSON.stringify(format.schema)).not.toContain('maxLength')
    })

    test('스키마 이름은 허용 문자만 남기고 64자로 자른다', async () => {
        let name = ''
        server.use(
            http.post(RESPONSES_URL, async ({ request }) => {
                name = ((await request.json()) as { text: { format: { name: string } } }).text.format.name
                return HttpResponse.json(responseBody(), { headers: REQUEST_ID_HEADERS })
            }),
        )

        await openaiAdapter.generateStructured({ ...generateInput(), schemaName: `weird name/with.chars${'x'.repeat(100)}` })

        expect(name).toMatch(/^[A-Za-z0-9_-]{64}$/)
        expect(name.startsWith('weird_name_with_chars')).toBe(true)
    })

    test('output_text 가 여러 조각이어도 이어 붙여 읽는다', async () => {
        const json = JSON.stringify(VALID_REPORT)
        server.use(
            replyResponse(
                responseBody({
                    output: [
                        messageItem([
                            { type: 'output_text', text: json.slice(0, 12) },
                            { type: 'output_text', text: json.slice(12) },
                        ]),
                    ],
                }),
            ),
        )

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({ ok: true, data: VALID_REPORT })
    })

    test('usage 가 없으면 0 으로 센다', async () => {
        server.use(replyResponse(responseBody({ usage: null })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({
            ok: true,
            usage: { inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 },
        })
    })
})

describe('openaiAdapter.generateStructured 응답 실패', () => {
    test('refusal 항목은 refused 이고 사용량을 남긴다', async () => {
        server.use(replyResponse(responseBody({ output: [messageItem([{ type: 'refusal', refusal: "I can't help with that." }])] })))

        const result = await openaiAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.REFUSED,
            requestId: REQUEST_ID,
            usage: { inputTokens: 120, outputTokens: 40, cachedInputTokens: 64 },
        })
        expect(findLeakedTexts(result, ["I can't help"])).toEqual([])
    })

    test('incomplete max_output_tokens 는 truncated 다', async () => {
        server.use(replyResponse(responseBody({ status: 'incomplete', incomplete_details: { reason: 'max_output_tokens' }, output: [] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.TRUNCATED,
            requestId: REQUEST_ID,
        })
    })

    test('incomplete content_filter 는 refused 다', async () => {
        server.use(replyResponse(responseBody({ status: 'incomplete', incomplete_details: { reason: 'content_filter' }, output: [] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.REFUSED })
    })

    test.each([
        ['rate_limit_exceeded', AI_FAILURE_KIND.RATE_LIMITED],
        ['server_error', AI_FAILURE_KIND.UNAVAILABLE],
        ['invalid_prompt', AI_FAILURE_KIND.REFUSED],
        ['something_new', AI_FAILURE_KIND.UNAVAILABLE],
    ])('status failed 의 error.code %s 는 %s 로 분류한다', async (code, kind) => {
        server.use(replyResponse(responseBody({ status: 'failed', error: { code, message: 'x' }, output: [] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind })
    })

    test.each(['in_progress', 'queued', 'cancelled'])('끝나지 않은 status %s 는 unavailable 이다', async (status) => {
        server.use(replyResponse(responseBody({ status, output: [] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.UNAVAILABLE })
    })

    test('JSON 이 아닌 본문은 invalid_output 이다', async () => {
        server.use(replyResponse(responseBody({ output: [messageItem([{ type: 'output_text', text: 'not json' }])] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_OUTPUT,
            usage: { outputTokens: 40 },
        })
    })

    test('공급자 스키마에서 뺀 제약도 원래 Zod 로 다시 검증해 거절한다', async () => {
        server.use(replyResponse(responseBody({ output: [messageItem([{ type: 'output_text', text: JSON.stringify(OVERSIZED_REPORT) }])] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('출력 항목이 비어 있으면 invalid_output 이다', async () => {
        server.use(replyResponse(responseBody({ output: [{ type: 'reasoning', summary: [] }] })))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('응답 봉투 형식이 틀리면 invalid_output 이다', async () => {
        server.use(replyResponse({ unexpected: true }))

        expect(await openaiAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_OUTPUT,
            usage: null,
        })
    })

    test('변환할 수 없는 스키마는 요청을 보내지 않고 bad_request 다', async () => {
        const result = await openaiAdapter.generateStructured({ ...generateInput(), schema: generateInput().schema.nullable() })

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.BAD_REQUEST, providerErrorType: 'schema_unsupported' })
    })
})

describe('openaiAdapter.generateStructured HTTP 오류', () => {
    test.each([
        [401, { type: 'invalid_request_error', code: 'invalid_api_key', message: 'Incorrect API key' }, {}, AI_FAILURE_KIND.INVALID_KEY],
        [403, { type: 'invalid_request_error', code: 'unsupported_country_region_territory', message: 'region' }, {}, AI_FAILURE_KIND.FORBIDDEN],
        [429, { type: 'insufficient_quota', code: 'insufficient_quota', message: 'quota' }, {}, AI_FAILURE_KIND.QUOTA],
        [429, { type: 'invalid_request_error', code: 'credit_balance_exhausted', message: 'credit' }, {}, AI_FAILURE_KIND.QUOTA],
        [429, { type: 'invalid_request_error', code: 'project_spend_limit_exceeded', message: 'cap' }, {}, AI_FAILURE_KIND.QUOTA],
        [
            429,
            { type: 'requests', code: 'rate_limit_exceeded', message: 'slow' },
            { 'retry-after': String(RETRY_AFTER_SECONDS) },
            AI_FAILURE_KIND.RATE_LIMITED,
        ],
        [400, { type: 'invalid_request_error', code: 'invalid_json_schema', message: 'schema' }, {}, AI_FAILURE_KIND.BAD_REQUEST],
        [404, { type: 'invalid_request_error', code: 'model_not_found', message: 'model' }, {}, AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [400, { type: 'invalid_request_error', code: 'model_not_found', message: 'model' }, {}, AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [500, { type: 'server_error', code: null, message: 'oops' }, {}, AI_FAILURE_KIND.UNAVAILABLE],
        [503, { type: 'server_error', code: 'server_is_overloaded', message: 'busy' }, {}, AI_FAILURE_KIND.UNAVAILABLE],
    ])('HTTP %d %o 는 %s 로 분류한다', async (status, error, headers, kind) => {
        server.use(replyError(status, error, headers))

        const result = await openaiAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind, httpStatus: status, requestId: REQUEST_ID, usage: null })
    })

    test('429 의 retry-after 와 오류 code 를 읽고 재시도하지 않는다', async () => {
        let callCount = 0
        server.use(
            http.post(RESPONSES_URL, () => {
                callCount += 1
                return HttpResponse.json(
                    { error: { type: 'requests', code: 'rate_limit_exceeded', message: 'slow' } },
                    { status: 429, headers: { ...REQUEST_ID_HEADERS, 'retry-after': String(RETRY_AFTER_SECONDS) } },
                )
            }),
        )

        const result = await openaiAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.RATE_LIMITED,
            retryAfterSeconds: RETRY_AFTER_SECONDS,
            providerErrorType: 'rate_limit_exceeded',
        })
        expect(callCount).toBe(1)
    })

    test('연결 실패는 unavailable 이고 메시지·URL·키를 결과에 남기지 않는다', async () => {
        const consoleSpies = watchConsole()
        server.use(http.post(RESPONSES_URL, () => HttpResponse.error()))

        const result = await openaiAdapter.generateStructured(generateInput())

        expect(result).toEqual({
            ok: false,
            kind: AI_FAILURE_KIND.UNAVAILABLE,
            requestId: null,
            httpStatus: null,
            providerErrorType: null,
            retryAfterSeconds: null,
            usage: null,
        })
        expect(findLeakedTexts(result, [FAKE_API_KEY, 'api.openai.com'])).toEqual([])
        consoleSpies.forEach((spy) => expect(spy).not.toHaveBeenCalled())
    })

    test('공급자 오류 본문이 키를 되풀이해도 결과·로그에 나오지 않는다', async () => {
        const consoleSpies = watchConsole()
        const echoed = `Incorrect API key provided: ${FAKE_API_KEY}`
        server.use(replyError(401, { type: 'invalid_request_error', code: 'invalid_api_key', message: echoed }))

        const result = await openaiAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_KEY, providerErrorType: 'invalid_api_key' })
        expect(findLeakedTexts(result, [FAKE_API_KEY, echoed])).toEqual([])
        consoleSpies.forEach((spy) => expect(spy).not.toHaveBeenCalled())
    })

    test('성공 결과에도 키가 없다', async () => {
        server.use(replyResponse(responseBody()))

        expect(findLeakedTexts(await openaiAdapter.generateStructured(generateInput()), [FAKE_API_KEY])).toEqual([])
    })
})

describe('openaiAdapter.validateKey', () => {
    test('GET /v1/models 가 200 이면 ok 다', async () => {
        const requests: { authorization: string | null; method: string }[] = []
        server.use(
            http.get(MODELS_URL, ({ request }) => {
                requests.push({ authorization: request.headers.get('authorization'), method: request.method })
                return HttpResponse.json({ object: 'list', data: [] })
            }),
        )

        expect(await openaiAdapter.validateKey(callInput())).toEqual({ ok: true })
        expect(requests).toEqual([{ authorization: `Bearer ${FAKE_API_KEY}`, method: 'GET' }])
    })

    test('401 은 invalid_key 다', async () => {
        server.use(
            http.get(MODELS_URL, () =>
                HttpResponse.json(
                    { error: { type: 'invalid_request_error', code: 'invalid_api_key', message: 'bad' } },
                    { status: 401, headers: REQUEST_ID_HEADERS },
                ),
            ),
        )

        expect(await openaiAdapter.validateKey(callInput())).toEqual({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_KEY,
            requestId: REQUEST_ID,
            httpStatus: 401,
            providerErrorType: 'invalid_api_key',
            retryAfterSeconds: null,
        })
    })

    test('연결 실패는 unavailable 이다', async () => {
        server.use(http.get(MODELS_URL, () => HttpResponse.error()))

        expect(await openaiAdapter.validateKey(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.UNAVAILABLE })
    })
})

describe('openaiAdapter.listModels', () => {
    test('응답 순서를 지키고 구조화 지원 여부는 알 수 없음으로 둔다', async () => {
        server.use(http.get(MODELS_URL, () => HttpResponse.json({ object: 'list', data: [{ id: 'gpt-b', object: 'model' }, { id: 'gpt-a' }] })))

        expect(await openaiAdapter.listModels(callInput())).toEqual({
            ok: true,
            models: [
                { id: 'gpt-b', label: 'gpt-b', structured: null },
                { id: 'gpt-a', label: 'gpt-a', structured: null },
            ],
        })
    })

    test('429 는 rate_limited 이고 retry-after 를 읽는다', async () => {
        server.use(
            http.get(MODELS_URL, () =>
                HttpResponse.json(
                    { error: { type: 'requests', code: 'rate_limit_exceeded', message: 'slow' } },
                    { status: 429, headers: { 'retry-after': '3' } },
                ),
            ),
        )

        expect(await openaiAdapter.listModels(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.RATE_LIMITED, retryAfterSeconds: 3 })
    })

    test('목록 형식이 틀리면 invalid_output 이다', async () => {
        server.use(http.get(MODELS_URL, () => HttpResponse.json({ models: [] })))

        expect(await openaiAdapter.listModels(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })
})
