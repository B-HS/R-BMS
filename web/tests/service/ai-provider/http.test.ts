import { describe, expect, test } from 'bun:test'
import { http, HttpResponse } from 'msw'
import {
    classifyHttpFailure,
    createFailureDetail,
    HTTP_STATUS,
    normalizeProviderErrorType,
    normalizeRequestId,
    parseRetryAfterSeconds,
    PROVIDER_ERROR_TYPE_MAX_LENGTH,
    REQUEST_ID_MAX_LENGTH,
    RETRY_AFTER_MAX_SECONDS,
    sendProviderRequest,
    tryParseJson,
} from '@server/service/shared/ai-provider/http'
import { AI_FAILURE_KIND } from '@server/dto/ai'
import { REQUEST_ID, server, setupMswServer } from './support'

const TEST_URL = 'https://provider.test/v1/echo'
const HTTP_DATE_NOW = Date.parse('Wed, 21 Oct 2026 07:28:00 GMT')
const HTTP_DATE_THIRTY_SECONDS_LATER = 'Wed, 21 Oct 2026 07:28:30 GMT'
const HTTP_DATE_BEFORE_NOW = 'Wed, 21 Oct 2026 07:27:00 GMT'

setupMswServer()

describe('parseRetryAfterSeconds', () => {
    test.each([
        ['17', 17],
        [' 5 ', 5],
        ['0', 0],
    ])('초 단위 %p 는 %d 다', (value, expected) => {
        expect(parseRetryAfterSeconds(value)).toBe(expected)
    })

    test('HTTP 날짜는 지금까지 남은 초로 바꾼다', () => {
        expect(parseRetryAfterSeconds(HTTP_DATE_THIRTY_SECONDS_LATER, HTTP_DATE_NOW)).toBe(30)
    })

    test('이미 지난 HTTP 날짜는 0 이다', () => {
        expect(parseRetryAfterSeconds(HTTP_DATE_BEFORE_NOW, HTTP_DATE_NOW)).toBe(0)
    })

    test('너무 큰 값은 상한으로 줄인다', () => {
        expect(parseRetryAfterSeconds('999999999')).toBe(RETRY_AFTER_MAX_SECONDS)
    })

    test.each([['abc'], [''], [null], [undefined]])('읽을 수 없는 값 %p 는 null 이다', (value) => {
        expect(parseRetryAfterSeconds(value)).toBeNull()
    })
})

describe('normalizeRequestId / normalizeProviderErrorType', () => {
    test('허용 문자만 있는 값은 앞뒤 공백을 지우고 그대로 쓴다', () => {
        expect(normalizeRequestId(` ${REQUEST_ID} `)).toBe(REQUEST_ID)
        expect(normalizeProviderErrorType('rate_limit_error')).toBe('rate_limit_error')
    })

    test.each([['has space'], ['semi;colon'], ['줄바꿈\nline'], [''], [null], [undefined]])('허용하지 않는 요청 id %p 는 null 이다', (value) => {
        expect(normalizeRequestId(value)).toBeNull()
    })

    test('길이 상한을 넘으면 null 이다', () => {
        expect(normalizeRequestId('a'.repeat(REQUEST_ID_MAX_LENGTH))).not.toBeNull()
        expect(normalizeRequestId('a'.repeat(REQUEST_ID_MAX_LENGTH + 1))).toBeNull()
        expect(normalizeProviderErrorType('a'.repeat(PROVIDER_ERROR_TYPE_MAX_LENGTH + 1))).toBeNull()
    })

    test('공백이 든 오류 종류는 null 이다', () => {
        expect(normalizeProviderErrorType('rate limit')).toBeNull()
    })
})

describe('classifyHttpFailure', () => {
    test.each([
        [HTTP_STATUS.UNAUTHORIZED, false, AI_FAILURE_KIND.INVALID_KEY],
        [HTTP_STATUS.FORBIDDEN, false, AI_FAILURE_KIND.FORBIDDEN],
        [HTTP_STATUS.PAYMENT_REQUIRED, false, AI_FAILURE_KIND.QUOTA],
        [HTTP_STATUS.TOO_MANY_REQUESTS, false, AI_FAILURE_KIND.RATE_LIMITED],
        [HTTP_STATUS.TOO_MANY_REQUESTS, true, AI_FAILURE_KIND.QUOTA],
        [HTTP_STATUS.BAD_REQUEST, false, AI_FAILURE_KIND.BAD_REQUEST],
        [HTTP_STATUS.BAD_REQUEST, true, AI_FAILURE_KIND.QUOTA],
        [HTTP_STATUS.REQUEST_TIMEOUT, false, AI_FAILURE_KIND.UNAVAILABLE],
        [HTTP_STATUS.INTERNAL_SERVER_ERROR, false, AI_FAILURE_KIND.UNAVAILABLE],
        [529, false, AI_FAILURE_KIND.UNAVAILABLE],
        [HTTP_STATUS.NOT_FOUND, false, AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [422, false, AI_FAILURE_KIND.BAD_REQUEST],
    ])('상태 %d 쿼터 신호 %p 는 %s 다', (status, quotaSignal, expected) => {
        expect(classifyHttpFailure({ status, quotaSignal, providerErrorType: null })).toBe(expected)
    })

    test.each([
        [HTTP_STATUS.BAD_REQUEST, 'model_not_found', AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [HTTP_STATUS.BAD_REQUEST, 'not_found_error', AI_FAILURE_KIND.MODEL_UNAVAILABLE],
        [HTTP_STATUS.BAD_REQUEST, 'invalid_request_error', AI_FAILURE_KIND.BAD_REQUEST],
        [HTTP_STATUS.UNAUTHORIZED, 'model_not_found', AI_FAILURE_KIND.INVALID_KEY],
    ])('상태 %d 오류 종류 %s 는 %s 다', (status, providerErrorType, expected) => {
        expect(classifyHttpFailure({ status, quotaSignal: false, providerErrorType })).toBe(expected)
    })
})

describe('tryParseJson / createFailureDetail', () => {
    test('JSON 을 읽고 실패하면 ok false 다', () => {
        expect(tryParseJson('{"a":1}')).toEqual({ ok: true, value: { a: 1 } })
        expect(tryParseJson('{a')).toEqual({ ok: false })
    })

    test('실패 상세의 기본값은 모두 null 이다', () => {
        expect(createFailureDetail(AI_FAILURE_KIND.QUOTA)).toEqual({
            kind: AI_FAILURE_KIND.QUOTA,
            requestId: null,
            httpStatus: null,
            providerErrorType: null,
            retryAfterSeconds: null,
        })
    })
})

describe('sendProviderRequest', () => {
    test('POST 는 JSON 본문과 content-type 을 보내고 GET 은 본문이 없다', async () => {
        const captured: { method: string; contentType: string | null; body: string }[] = []
        server.use(
            http.all(TEST_URL, async ({ request }) => {
                captured.push({ method: request.method, contentType: request.headers.get('content-type'), body: await request.text() })
                return HttpResponse.json({ ok: true })
            }),
        )
        const signal = new AbortController().signal

        await sendProviderRequest({ url: TEST_URL, method: 'POST', headers: {}, body: { a: 1 }, signal, requestIdHeader: null })
        await sendProviderRequest({ url: TEST_URL, method: 'GET', headers: {}, signal, requestIdHeader: null })

        expect(captured).toEqual([
            { method: 'POST', contentType: 'application/json', body: '{"a":1}' },
            { method: 'GET', contentType: null, body: '' },
        ])
    })

    test('JSON 이 아닌 본문은 body 를 undefined 로 두고 상태는 그대로 돌려준다', async () => {
        server.use(http.get(TEST_URL, () => new HttpResponse('<html>bad gateway</html>', { status: 502 })))

        const response = await sendProviderRequest({
            url: TEST_URL,
            method: 'GET',
            headers: {},
            signal: new AbortController().signal,
            requestIdHeader: null,
        })

        expect(response).toEqual({ status: 502, ok: false, requestId: null, retryAfterSeconds: null, body: undefined })
    })

    test('형식이 이상한 요청 id 헤더는 버린다', async () => {
        server.use(http.get(TEST_URL, () => HttpResponse.json({}, { headers: { 'x-request-id': 'bad id; drop table' } })))

        const response = await sendProviderRequest({
            url: TEST_URL,
            method: 'GET',
            headers: {},
            signal: new AbortController().signal,
            requestIdHeader: 'x-request-id',
        })

        expect(response).toMatchObject({ ok: true, requestId: null })
    })

    test('연결 실패는 null 이다', async () => {
        server.use(http.get(TEST_URL, () => HttpResponse.error()))

        expect(
            await sendProviderRequest({ url: TEST_URL, method: 'GET', headers: {}, signal: new AbortController().signal, requestIdHeader: null }),
        ).toBeNull()
    })
})
