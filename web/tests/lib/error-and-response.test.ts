import { describe, expect, test } from 'bun:test'
import { ERROR_CODE } from '@server/lib/error-code'
import { createAppError, getStatusCode, isAppError } from '@server/lib/error'
import { errorResponse, paginatedResponse, publicErrorDetails, successResponse } from '@server/lib/api-response'
import { consumeRateLimit, resetRateLimits } from '@server/lib/with-rate-limit'
import { generateApiToken, hashApiToken, parseBearerToken, TOKEN_PREFIX } from '@server/lib/token'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'

describe('error 3파일 중앙화', () => {
    test('IR 코드가 api-spec 의 상태코드로 매핑된다', () => {
        expect(getStatusCode(ERROR_CODE.VALIDATION_ERROR)).toBe(400)
        expect(getStatusCode(ERROR_CODE.UNAUTHORIZED)).toBe(401)
        expect(getStatusCode(ERROR_CODE.FORBIDDEN)).toBe(403)
        expect(getStatusCode(ERROR_CODE.IR_CHART_NOT_FOUND)).toBe(404)
        expect(getStatusCode(ERROR_CODE.IR_SETTING_NOT_FOUND)).toBe(404)
        expect(getStatusCode(ERROR_CODE.IR_ACCOUNT_EXISTS)).toBe(409)
        expect(getStatusCode(ERROR_CODE.IR_PAYLOAD_TOO_LARGE)).toBe(413)
        expect(getStatusCode(ERROR_CODE.RATE_LIMITED)).toBe(429)
        expect(getStatusCode(ERROR_CODE.SERVICE_NOT_CONFIGURED)).toBe(503)
    })

    test('createAppError 는 메시지와 상태코드를 함께 담는다', () => {
        const error = createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId: 'alice' })
        expect(error.statusCode).toBe(404)
        expect(error.message.length).toBeGreaterThan(0)
        expect(error.details).toEqual({ playerId: 'alice' })
        expect(isAppError(error)).toBe(true)
    })

    test('isAppError 는 일반 Error 를 걸러낸다', () => {
        expect(isAppError(new Error('boom'))).toBe(false)
        expect(isAppError(null)).toBe(false)
    })
})

describe('api-response 헬퍼', () => {
    test('successResponse 는 봉투를 씌운다', () => {
        expect(successResponse({ a: 1 })).toEqual({ success: true, data: { a: 1 } })
    })

    test('paginatedResponse 는 totalPages 를 계산한다', () => {
        const result = paginatedResponse([1, 2], { page: 1, limit: 50, total: 1234 })
        expect(result.pagination.totalPages).toBe(25)
    })

    test('총 개수가 0 이어도 totalPages 는 최소 1 이다', () => {
        expect(paginatedResponse([], { page: 1, limit: 50, total: 0 }).pagination.totalPages).toBe(1)
    })

    test('errorResponse 는 code 와 message 를 담는다', () => {
        const result = errorResponse(ERROR_CODE.UNAUTHORIZED, '인증이 필요합니다.')
        expect(result.success).toBe(false)
        expect(result.error.code).toBe('UNAUTHORIZED')
    })

    test('publicErrorDetails 는 프로덕션에서 문자열 field·reason 만 남긴다', () => {
        expect(publicErrorDetails({ field: 'judge', reason: 'x', issues: [1], player: 'p' }, true)).toEqual({ field: 'judge', reason: 'x' })
        expect(publicErrorDetails({ field: 'judge', reason: 3 }, true)).toEqual({ field: 'judge' })
    })

    test('프로덕션에서도 AI 오류의 provider·request_id·retry_after_seconds 는 남기고 나머지는 버린다', () => {
        const details = {
            reason: 'rate_limited',
            provider: 'anthropic',
            request_id: 'req_1',
            retry_after_seconds: 17,
            http_status: 429,
            provider_error_type: 'rate_limit_error',
            api_key: 'sk-secret',
            body: 'provider body',
        }
        expect(publicErrorDetails(details, true)).toEqual({
            reason: 'rate_limited',
            provider: 'anthropic',
            request_id: 'req_1',
            retry_after_seconds: 17,
        })
        expect(publicErrorDetails({ retry_after_seconds: '17', request_id: 5, provider: null }, true)).toBeUndefined()
    })

    test('프로덕션에서 field·reason 이 없으면 details 를 싣지 않는다', () => {
        expect(publicErrorDetails({ player: 'p' }, true)).toBeUndefined()
        expect(publicErrorDetails(undefined, true)).toBeUndefined()
        expect(publicErrorDetails({ field: 3, reason: null }, true)).toBeUndefined()
    })

    test('비프로덕션에서는 details 를 그대로 둔다', () => {
        const details = { issues: [1] }
        expect(publicErrorDetails(details, false)).toBe(details)
        expect(publicErrorDetails(undefined, false)).toBeUndefined()
        expect(errorResponse(ERROR_CODE.VALIDATION_ERROR, 'x', details).error).toEqual({ code: 'VALIDATION_ERROR', message: 'x', details })
        expect('details' in errorResponse(ERROR_CODE.VALIDATION_ERROR, 'x').error).toBe(false)
    })
})

describe('api_token 해시 / Bearer 파싱', () => {
    test('발급 토큰은 rbms_ 접두사를 가진다', () => {
        expect(generateApiToken().startsWith(TOKEN_PREFIX)).toBe(true)
    })

    test('같은 평문은 같은 해시, 다른 평문은 다른 해시다', () => {
        expect(hashApiToken('rbms_abc')).toBe(hashApiToken('rbms_abc'))
        expect(hashApiToken('rbms_abc')).not.toBe(hashApiToken('rbms_abd'))
        expect(hashApiToken('rbms_abc')).toHaveLength(64)
    })

    test('Authorization 헤더에서 Bearer 토큰만 뽑는다', () => {
        expect(parseBearerToken(new Request('http://x/', { headers: { authorization: 'Bearer tok-123' } }))).toBe('tok-123')
        expect(parseBearerToken(new Request('http://x/', { headers: { authorization: 'bearer tok-123' } }))).toBe('tok-123')
        expect(parseBearerToken(new Request('http://x/', { headers: { authorization: 'Basic tok-123' } }))).toBeNull()
        expect(parseBearerToken(new Request('http://x/'))).toBeNull()
    })
})

describe('withRateLimit 인메모리 카운터', () => {
    test('한도까지 통과하고 그 다음부터 차단한다', () => {
        resetRateLimits()
        const now = 1_000_000
        expect(consumeRateLimit('k', 3, 60_000, now).allowed).toBe(true)
        expect(consumeRateLimit('k', 3, 60_000, now).allowed).toBe(true)
        expect(consumeRateLimit('k', 3, 60_000, now).allowed).toBe(true)
        expect(consumeRateLimit('k', 3, 60_000, now).allowed).toBe(false)
    })

    test('윈도가 지나면 카운터가 초기화된다', () => {
        resetRateLimits()
        const now = 2_000_000
        consumeRateLimit('w', 1, 60_000, now)
        expect(consumeRateLimit('w', 1, 60_000, now).allowed).toBe(false)
        expect(consumeRateLimit('w', 1, 60_000, now + 60_001).allowed).toBe(true)
    })

    test('키가 다르면 서로 영향을 주지 않는다', () => {
        resetRateLimits()
        const now = 3_000_000
        consumeRateLimit('a', 1, 60_000, now)
        expect(consumeRateLimit('b', 1, 60_000, now).allowed).toBe(true)
    })
})

describe('classifyChartHash', () => {
    test('32자는 md5, 64자는 sha256 으로 판별한다', () => {
        expect(classifyChartHash('a'.repeat(32))).toBe('md5')
        expect(classifyChartHash('a'.repeat(64))).toBe('sha256')
    })

    test('그 외 길이는 unknown 이다', () => {
        expect(classifyChartHash('abc')).toBe('unknown')
        expect(classifyChartHash('')).toBe('unknown')
        expect(classifyChartHash('a'.repeat(63))).toBe('unknown')
    })
})
