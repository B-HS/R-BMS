import { describe, expect, test } from 'bun:test'
import { http, HttpResponse, type JsonBodyType } from 'msw'
import { listJsonSchemaViolations } from '@server/service/shared/ai-provider/json-schema'
import { ollamaAdapter, parseLenientJson } from '@server/service/shared/ai-provider/ollama'
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
    RETRY_AFTER_SECONDS,
    server,
    SYSTEM_PROMPT,
    setupMswServer,
    USER_INPUT,
    VALID_REPORT,
    watchConsole,
} from './support'

const CHAT_URL = 'https://ollama.com/api/chat'
const TAGS_URL = 'https://ollama.com/api/tags'
const BALANCE_URL = 'https://ollama.com/api/balance'

const chatBody = (overrides: Record<string, unknown> = {}) => ({
    model: MODEL_ID,
    created_at: '2026-10-19T00:00:00Z',
    message: { role: 'assistant', content: JSON.stringify(VALID_REPORT) },
    done: true,
    done_reason: 'stop',
    prompt_eval_count: 90,
    prompt_eval_cached_count: 30,
    eval_count: 35,
    ...overrides,
})

const withContent = (content: string) => chatBody({ message: { role: 'assistant', content } })

const replyChat = (body: JsonBodyType, status = 200, headers: Record<string, string> = {}) =>
    http.post(CHAT_URL, () => HttpResponse.json(body, { status, headers }))

setupMswServer()

describe('ollamaAdapter 메타', () => {
    test('공급자 id 와 형식 보장 등급이 계약과 같다', () => {
        expect(ollamaAdapter.id).toBe('ollama')
        expect(ollamaAdapter.strictness).toBe(AI_STRICTNESS.BEST_EFFORT)
    })
})

describe('ollamaAdapter.generateStructured 성공', () => {
    test('스키마를 시스템 프롬프트에 싣고 비스트리밍으로 보내 사용량을 정규화한다', async () => {
        const requests: { headers: Headers; body: Record<string, unknown>; redirect: RequestRedirect }[] = []
        server.use(
            http.post(CHAT_URL, async ({ request }) => {
                requests.push({ headers: request.headers, body: await readJsonBody(request), redirect: request.redirect })
                return HttpResponse.json(chatBody())
            }),
        )

        const result = await ollamaAdapter.generateStructured(generateInput())

        expect(result).toEqual({
            ok: true,
            data: VALID_REPORT,
            usage: { inputTokens: 90, outputTokens: 35, cachedInputTokens: 30 },
            requestId: null,
        })
        expect(requests[0].headers.get('authorization')).toBe(`Bearer ${FAKE_API_KEY}`)
        expect(requests[0].redirect).toBe('error')
        expect(requests[0].body).toMatchObject({ model: MODEL_ID, stream: false, options: { num_predict: MAX_OUTPUT_TOKENS } })
        expect(requests[0].body).not.toHaveProperty('format')
        const messages = requests[0].body.messages as { role: string; content: string }[]
        expect(messages[0].role).toBe('system')
        expect(messages[0].content.startsWith(SYSTEM_PROMPT)).toBe(true)
        expect(messages[1]).toEqual({ role: 'user', content: USER_INPUT })
        const embeddedSchema = JSON.parse(messages[0].content.slice(messages[0].content.indexOf('{"type":"object"')))
        expect(listJsonSchemaViolations(embeddedSchema)).toEqual([])
        expect(JSON.stringify(embeddedSchema)).not.toContain('maxLength')
    })

    test('코드 펜스로 감싼 JSON 도 읽는다', async () => {
        server.use(replyChat(withContent(`\`\`\`json\n${JSON.stringify(VALID_REPORT)}\n\`\`\``)))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: true, data: VALID_REPORT })
    })

    test('앞뒤 설명이 붙은 JSON 도 읽는다', async () => {
        server.use(replyChat(withContent(`Sure! Here is the report: ${JSON.stringify(VALID_REPORT)} Hope it helps.`)))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: true, data: VALID_REPORT })
    })

    test('think 블록 안의 중괄호는 무시한다', async () => {
        server.use(replyChat(withContent(`<think>maybe {"summary": 1}</think>${JSON.stringify(VALID_REPORT)}`)))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: true, data: VALID_REPORT })
    })

    test('사용량 필드가 없으면 0 으로 센다', async () => {
        server.use(replyChat(chatBody({ prompt_eval_count: undefined, prompt_eval_cached_count: undefined, eval_count: undefined })))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({
            ok: true,
            usage: { inputTokens: 0, outputTokens: 0, cachedInputTokens: 0 },
        })
    })
})

describe('ollamaAdapter.generateStructured 응답 실패', () => {
    test('done_reason length 는 truncated 이고 사용량을 남긴다', async () => {
        server.use(replyChat(chatBody({ done_reason: 'length', message: { role: 'assistant', content: '{"summary":"cut' } })))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.TRUNCATED,
            requestId: null,
            usage: { inputTokens: 90, outputTokens: 35, cachedInputTokens: 30 },
        })
    })

    test('정상 종료가 아닌 done_reason 은 truncated 다', async () => {
        server.use(replyChat(chatBody({ done_reason: 'load' })))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.TRUNCATED })
    })

    test('done 이 false 면 invalid_output 이다', async () => {
        server.use(replyChat(chatBody({ done: false })))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('JSON 이 없는 본문은 invalid_output 이다', async () => {
        server.use(replyChat(withContent('I could not produce a report.')))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_OUTPUT,
            usage: { outputTokens: 35 },
        })
    })

    test('스키마와 다른 JSON 은 invalid_output 이다', async () => {
        server.use(replyChat(withContent(JSON.stringify({ summary: 'only summary' }))))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('공급자 스키마에서 뺀 제약도 원래 Zod 로 다시 검증해 거절한다', async () => {
        server.use(replyChat(withContent(JSON.stringify(OVERSIZED_REPORT))))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })

    test('응답 봉투 형식이 틀리면 invalid_output 이다', async () => {
        server.use(replyChat({ unexpected: true }))

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_OUTPUT,
            usage: null,
        })
    })

    test('변환할 수 없는 스키마는 요청을 보내지 않고 bad_request 다', async () => {
        const result = await ollamaAdapter.generateStructured({ ...generateInput(), schema: generateInput().schema.nullable() })

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.BAD_REQUEST, providerErrorType: 'schema_unsupported' })
    })
})

describe('ollamaAdapter.generateStructured HTTP 오류', () => {
    test.each([
        [401, {}, AI_FAILURE_KIND.INVALID_KEY],
        [403, {}, AI_FAILURE_KIND.FORBIDDEN],
        [429, { 'retry-after': String(RETRY_AFTER_SECONDS) }, AI_FAILURE_KIND.RATE_LIMITED],
        [400, {}, AI_FAILURE_KIND.BAD_REQUEST],
        [404, {}, AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [500, {}, AI_FAILURE_KIND.UNAVAILABLE],
        [502, {}, AI_FAILURE_KIND.UNAVAILABLE],
        [503, {}, AI_FAILURE_KIND.UNAVAILABLE],
    ])('HTTP %d 는 %s 로 분류한다', async (status, headers, kind) => {
        server.use(replyChat({ error: 'provider says no' }, status, headers))

        const result = await ollamaAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind, httpStatus: status, requestId: null, usage: null })
        expect(findLeakedTexts(result, ['provider says no'])).toEqual([])
    })

    test('429 의 retry-after 는 초 단위로 읽기만 하고 재시도하지 않는다', async () => {
        let callCount = 0
        server.use(
            http.post(CHAT_URL, () => {
                callCount += 1
                return HttpResponse.json({ error: 'busy' }, { status: 429, headers: { 'retry-after': String(RETRY_AFTER_SECONDS) } })
            }),
        )

        expect(await ollamaAdapter.generateStructured(generateInput())).toMatchObject({
            ok: false,
            kind: AI_FAILURE_KIND.RATE_LIMITED,
            retryAfterSeconds: RETRY_AFTER_SECONDS,
        })
        expect(callCount).toBe(1)
    })

    test('연결 실패는 unavailable 이고 메시지·URL·키를 결과에 남기지 않는다', async () => {
        const consoleSpies = watchConsole()
        server.use(http.post(CHAT_URL, () => HttpResponse.error()))

        const result = await ollamaAdapter.generateStructured(generateInput())

        expect(result).toEqual({
            ok: false,
            kind: AI_FAILURE_KIND.UNAVAILABLE,
            requestId: null,
            httpStatus: null,
            providerErrorType: null,
            retryAfterSeconds: null,
            usage: null,
        })
        expect(findLeakedTexts(result, [FAKE_API_KEY, 'ollama.com'])).toEqual([])
        consoleSpies.forEach((spy) => expect(spy).not.toHaveBeenCalled())
    })

    test('공급자 오류 본문이 키를 되풀이해도 결과·로그에 나오지 않는다', async () => {
        const consoleSpies = watchConsole()
        server.use(replyChat({ error: `unauthorized: ${FAKE_API_KEY}` }, 401))

        const result = await ollamaAdapter.generateStructured(generateInput())

        expect(result).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_KEY })
        expect(findLeakedTexts(result, [FAKE_API_KEY])).toEqual([])
        consoleSpies.forEach((spy) => expect(spy).not.toHaveBeenCalled())
    })

    test('성공 결과에도 키가 없다', async () => {
        server.use(replyChat(chatBody()))

        expect(findLeakedTexts(await ollamaAdapter.generateStructured(generateInput()), [FAKE_API_KEY])).toEqual([])
    })
})

describe('ollamaAdapter.validateKey', () => {
    test('GET /api/balance 가 200 이면 ok 다', async () => {
        const requests: { authorization: string | null; method: string }[] = []
        server.use(
            http.get(BALANCE_URL, ({ request }) => {
                requests.push({ authorization: request.headers.get('authorization'), method: request.method })
                return HttpResponse.json({ included: { balance_usd: 1 }, purchased: { balance_usd: 0 } })
            }),
        )

        expect(await ollamaAdapter.validateKey(callInput())).toEqual({ ok: true })
        expect(requests).toEqual([{ authorization: `Bearer ${FAKE_API_KEY}`, method: 'GET' }])
    })

    test('401 은 invalid_key 다', async () => {
        server.use(http.get(BALANCE_URL, () => HttpResponse.json({ error: 'Invalid credentials' }, { status: 401 })))

        expect(await ollamaAdapter.validateKey(callInput())).toEqual({
            ok: false,
            kind: AI_FAILURE_KIND.INVALID_KEY,
            requestId: null,
            httpStatus: 401,
            providerErrorType: null,
            retryAfterSeconds: null,
        })
    })

    test('분당 10회 제한(429)은 rate_limited 이고 retry-after 를 읽는다', async () => {
        server.use(http.get(BALANCE_URL, () => HttpResponse.json({ error: 'slow' }, { status: 429, headers: { 'retry-after': '30' } })))

        expect(await ollamaAdapter.validateKey(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.RATE_LIMITED, retryAfterSeconds: 30 })
    })

    test('연결 실패는 unavailable 이다', async () => {
        server.use(http.get(BALANCE_URL, () => HttpResponse.error()))

        expect(await ollamaAdapter.validateKey(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.UNAVAILABLE })
    })
})

describe('ollamaAdapter.listModels', () => {
    test('GET /api/tags 의 name 을 id 로 쓰고 구조화 지원 여부는 알 수 없음으로 둔다', async () => {
        server.use(
            http.get(TAGS_URL, () => HttpResponse.json({ models: [{ name: 'gemma4:31b', model: 'gemma4:31b', size: 1 }, { name: 'gpt-oss:120b' }] })),
        )

        expect(await ollamaAdapter.listModels(callInput())).toEqual({
            ok: true,
            models: [
                { id: 'gemma4:31b', label: 'gemma4:31b', structured: null },
                { id: 'gpt-oss:120b', label: 'gpt-oss:120b', structured: null },
            ],
        })
    })

    test('401 은 invalid_key 다', async () => {
        server.use(http.get(TAGS_URL, () => HttpResponse.json({ error: 'nope' }, { status: 401 })))

        expect(await ollamaAdapter.listModels(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_KEY })
    })

    test('목록 형식이 틀리면 invalid_output 이다', async () => {
        server.use(http.get(TAGS_URL, () => HttpResponse.json({ data: [] })))

        expect(await ollamaAdapter.listModels(callInput())).toMatchObject({ ok: false, kind: AI_FAILURE_KIND.INVALID_OUTPUT })
    })
})

describe('parseLenientJson', () => {
    test('문자열 안의 중괄호와 이스케이프된 따옴표에 속지 않는다', () => {
        const object = { summary: 'brace } and quote " inside', nested: { a: 1 } }

        expect(parseLenientJson(`text ${JSON.stringify(object)} tail`)).toEqual({ ok: true, value: object })
    })

    test('JSON 바깥 설명에 짝 없는 따옴표가 있어도 객체를 찾는다', () => {
        expect(parseLenientJson('He said "hello and then: {"a":1}')).toEqual({ ok: true, value: { a: 1 } })
    })

    test('JSON 객체가 여러 개면 처음 파싱되는 것을 쓴다', () => {
        expect(parseLenientJson('first {"a":1} then {"b":2}')).toEqual({ ok: true, value: { a: 1 } })
    })

    test('균형이 맞지 않는 중괄호만 있으면 실패한다', () => {
        expect(parseLenientJson('{"a": 1')).toEqual({ ok: false })
    })

    test('빈 문자열은 실패한다', () => {
        expect(parseLenientJson('')).toEqual({ ok: false })
    })
})
