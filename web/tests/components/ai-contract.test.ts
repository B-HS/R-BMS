import { afterEach, describe, expect, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import {
    AI_FAILURE_KIND,
    AI_LIMIT,
    AI_PROVIDER_IDS,
    aiCredentialSchema,
    aiModelSchema,
    aiOperatorLimitsSchema,
    aiUsageSummarySchema,
    aiUserSettingSchema,
} from '../../src/server/dto/ai'
import { config as proxyConfig } from '../../src/proxy'
import {
    aiCredentialListQueryOptions,
    aiModelListQueryOptions,
    aiOperatorLimitsQueryOptions,
    aiUsageSummaryQueryOptions,
    aiUserSettingQueryOptions,
} from '../../src/entities/ai.query'
import {
    AI_ERROR_CODE,
    AI_FORM_LIMIT,
    AI_PROVIDER_LABEL,
    AI_PROVIDER_NOTE,
    AI_PROVIDER_ORDER,
    selectActiveAiCredential,
    toAiFailureView,
    type AiCredential,
} from '../../src/entities/ai.type'
import { ERROR_CODE } from '../../src/server/lib/error-code'
import { removeUserScopedQueries } from '../../src/entities/auth.query'
import { MESSAGES } from '../../src/shared/i18n/messages'
import { parseLimitInput } from '../../src/features/ai/parse-limit-input'
import { AI_SETTINGS_HREF, NAV_ITEMS } from '../../src/shared/constants/navigation'
import { QUERY_KEY } from '../../src/shared/constants/query-key'
import { ApiError } from '../../src/shared/lib/fetch'

const originalFetch = globalThis.fetch

const capture = (data: unknown, status = 200) => {
    const calls: { url: string; method: string | undefined }[] = []
    globalThis.fetch = Object.assign(async (input: RequestInfo | URL, init?: RequestInit) => {
        calls.push({ url: String(input), method: init?.method })
        return new Response(JSON.stringify({ success: true, data }), { status, headers: { 'content-type': 'application/json' } })
    }, originalFetch)
    return calls
}

const run = async <T>(options: { queryFn?: unknown }) => (options.queryFn as () => Promise<T>)()

const credential = (provider: AiCredential['provider'], createdAt: number): AiCredential => ({
    provider,
    key_hint: 'abcd',
    model: null,
    created_at: createdAt,
    last_validated_at: null,
    last_error_kind: null,
})

afterEach(() => {
    globalThis.fetch = originalFetch
})

describe('AI 화면과 서버의 계약', () => {
    test('공급자 순서와 이름표가 서버의 공급자 목록과 같다', () => {
        expect([...AI_PROVIDER_ORDER].toSorted()).toEqual([...AI_PROVIDER_IDS].toSorted())
        expect(Object.keys(AI_PROVIDER_LABEL).toSorted()).toEqual([...AI_PROVIDER_IDS].toSorted())
        expect(Object.keys(AI_PROVIDER_NOTE).toSorted()).toEqual([...AI_PROVIDER_IDS].toSorted())
    })

    test('실패 종류 이름표가 서버의 실패 종류와 같다', () => {
        expect(Object.keys(MESSAGES.ko.pages.ai.failureKind).toSorted()).toEqual(Object.values(AI_FAILURE_KIND).toSorted())
    })

    test('화면의 입력 한도가 서버 한도와 같다', () => {
        expect(AI_FORM_LIMIT).toEqual({
            API_KEY_MIN_LENGTH: AI_LIMIT.API_KEY_MIN_LENGTH,
            API_KEY_MAX_LENGTH: AI_LIMIT.API_KEY_MAX_LENGTH,
            DAILY_CALLS_MAX: AI_LIMIT.DAILY_CALLS_MAX,
            CONCURRENT_RUNS_MAX: AI_LIMIT.CONCURRENT_RUNS_MAX,
            MONTHLY_TOKEN_LIMIT_MAX: AI_LIMIT.MONTHLY_TOKEN_LIMIT_MAX,
            MODEL_MAX_LENGTH: AI_LIMIT.MODEL_MAX_LENGTH,
        })
    })

    test('화면이 반응하는 AI 오류 코드가 서버 오류 코드에 있다', () => {
        for (const code of Object.values(AI_ERROR_CODE)) expect(ERROR_CODE[code]).toBe(code)
    })

    test('화면 테스트가 쓰는 응답 예시가 서버 출력 스키마를 통과한다', () => {
        const totals = { calls: 1, input_tokens: 2, output_tokens: 3, cached_input_tokens: 0 }
        expect(aiCredentialSchema.safeParse(credential('openai', 1)).success).toBe(true)
        expect(aiModelSchema.safeParse({ id: 'm', label: 'M', structured: null }).success).toBe(true)
        expect(aiUserSettingSchema.safeParse({ monthly_token_limit: null, default_provider: null, updated_at: null }).success).toBe(true)
        expect(
            aiOperatorLimitsSchema.safeParse({ daily_calls: 30, concurrent_runs: 1, default_monthly_token_limit: null, updated_at: null }).success,
        ).toBe(true)
        expect(
            aiUsageSummarySchema.safeParse({
                day: '2026-10-19',
                month: '2026-10',
                today: totals,
                this_month: totals,
                limits: { daily_calls: 30, concurrent_runs: 1, monthly_token_limit: null },
            }).success,
        ).toBe(true)
    })
})

describe('로그인·로그아웃 때 지우는 캐시', () => {
    test('AI 캐시와 운영자 AI 한도 캐시도 사용자 범위로 지운다', () => {
        const queryClient = new QueryClient()
        queryClient.setQueryData(QUERY_KEY.AI.CREDENTIALS, [])
        queryClient.setQueryData(QUERY_KEY.AI.MODELS('openai'), [])
        queryClient.setQueryData(QUERY_KEY.AI.USAGE, {})
        queryClient.setQueryData(QUERY_KEY.AI.SETTINGS, {})
        queryClient.setQueryData(QUERY_KEY.ADMIN.AI_LIMITS, {})
        queryClient.setQueryData(QUERY_KEY.STATS.SUMMARY, {})
        removeUserScopedQueries(queryClient)
        const keys = queryClient
            .getQueryCache()
            .getAll()
            .map((query) => query.queryKey)
        expect(keys).toEqual([QUERY_KEY.STATS.SUMMARY])
    })
})

describe('AI 실패 표시', () => {
    test('공급자가 알려 준 대기 초를 꺼내고 설정에서 고칠 수 있는 실패를 가린다', () => {
        const limited = new ApiError('AI_PROVIDER_RATE_LIMITED', '잠시 뒤', 429, { reason: 'rate_limited', retry_after_seconds: 17 })
        expect(toAiFailureView(limited)).toEqual({
            message: '잠시 뒤',
            retryAfterSeconds: 17,
            needsSettings: false,
            code: 'AI_PROVIDER_RATE_LIMITED',
        })
        expect(toAiFailureView(new ApiError('AI_MODEL_UNAVAILABLE', '모델', 422, { reason: 'model_unavailable' }))).toMatchObject({
            needsSettings: true,
            retryAfterSeconds: null,
        })
        expect(toAiFailureView(new ApiError('VALIDATION_ERROR', '모델', 400, { reason: 'model_not_selected' })).needsSettings).toBe(true)
        expect(toAiFailureView(new ApiError('AI_PROVIDER_RATE_LIMITED', 'x', 429, { retry_after_seconds: '17' })).retryAfterSeconds).toBeNull()
        expect(toAiFailureView(new TypeError('network'))).toEqual({ message: null, retryAfterSeconds: null, needsSettings: false, code: null })
    })
})

describe('사용 중인 AI 키 고르기', () => {
    test('기본 공급자에 키가 있으면 그것을 쓴다', () => {
        const credentials = [credential('anthropic', 1), credential('openai', 2)]
        expect(selectActiveAiCredential(credentials, 'openai')?.provider).toBe('openai')
    })

    test('기본 공급자가 없거나 키가 없으면 가장 먼저 등록한 키를 쓴다', () => {
        const credentials = [credential('ollama', 30), credential('openai', 10), credential('anthropic', 20)]
        expect(selectActiveAiCredential(credentials, null)?.provider).toBe('openai')
        expect(selectActiveAiCredential([credential('ollama', 30), credential('anthropic', 20)], 'openai')?.provider).toBe('anthropic')
    })

    test('키가 하나도 없으면 null 이다', () => {
        expect(selectActiveAiCredential([], 'openai')).toBeNull()
    })

    test('입력 배열의 순서를 바꾸지 않는다', () => {
        const credentials = [credential('ollama', 30), credential('openai', 10)]
        selectActiveAiCredential(credentials, null)
        expect(credentials.map((item) => item.provider)).toEqual(['ollama', 'openai'])
    })
})

describe('AI entities 쿼리가 실제 라우트 주소와 응답 형태를 따른다', () => {
    test('키 목록은 봉투 안의 배열을 벗겨 돌려준다', async () => {
        const calls = capture([credential('openai', 1)])
        const rows = await run<AiCredential[]>(aiCredentialListQueryOptions())
        expect(rows[0].provider).toBe('openai')
        expect(calls[0].url).toBe('/api/fe/me/ai/credentials')
    })

    test('모델 목록은 공급자별 주소와 쿼리 키를 쓰고 재시도하지 않는다', async () => {
        const calls = capture([{ id: 'm', label: 'M', structured: null }])
        const options = aiModelListQueryOptions('ollama')
        expect(await run<unknown[]>(options)).toHaveLength(1)
        expect(calls[0].url).toBe('/api/fe/me/ai/credentials/ollama/models')
        expect([...options.queryKey]).toEqual(QUERY_KEY.AI.MODELS('ollama'))
        expect([...options.queryKey]).not.toEqual([...aiModelListQueryOptions('openai').queryKey])
        expect(options.retry).toBe(false)
    })

    test('사용량, 사용자 설정, 운영자 한도는 각자의 주소를 부른다', async () => {
        const calls = capture({})
        await run(aiUsageSummaryQueryOptions())
        await run(aiUserSettingQueryOptions())
        await run(aiOperatorLimitsQueryOptions())
        expect(calls.map((call) => call.url)).toEqual(['/api/fe/me/ai/usage', '/api/fe/me/ai/settings', '/api/admin/ai/limits'])
        expect([...aiOperatorLimitsQueryOptions().queryKey]).toEqual([...QUERY_KEY.ADMIN.AI_LIMITS])
    })

    test('서버가 실패 봉투를 주면 ApiError 로 코드와 메시지를 남긴다', async () => {
        globalThis.fetch = Object.assign(
            async () =>
                new Response(JSON.stringify({ success: false, error: { code: 'AI_CREDENTIAL_NOT_FOUND', message: '등록된 AI 키가 없습니다.' } }), {
                    status: 404,
                    headers: { 'content-type': 'application/json' },
                }),
            originalFetch,
        )
        const error = await run(aiCredentialListQueryOptions()).catch((caught: unknown) => caught)
        expect(error).toBeInstanceOf(ApiError)
        expect([(error as ApiError).code, (error as ApiError).status]).toEqual(['AI_CREDENTIAL_NOT_FOUND', 404])
    })
})

describe('AI 진입점 상수', () => {
    test('내비게이션에 AI 가 있고 설정의 AI 탭 주소가 고정되어 있다', () => {
        expect(NAV_ITEMS.some((item) => item.href === '/ai' && item.labelKey === 'ai')).toBe(true)
        expect(AI_SETTINGS_HREF).toBe('/settings?tab=ai')
    })

    test('proxy 가 /ai 와 하위 경로를 세션 쿠키 검사 경로에 넣는다', () => {
        expect(proxyConfig.matcher).toContain('/ai/:path*')
    })

    test('쿼리 키는 AI 아래 계층 구조다', () => {
        expect(QUERY_KEY.AI.CREDENTIALS).toEqual(['ai', 'credentials'])
        expect(QUERY_KEY.AI.MODELS('openai')).toEqual(['ai', 'models', 'openai'])
        expect(QUERY_KEY.AI.USAGE.slice(0, 1)).toEqual([...QUERY_KEY.AI.ALL])
        expect(QUERY_KEY.AI.SETTINGS.slice(0, 1)).toEqual([...QUERY_KEY.AI.ALL])
    })
})

describe('한도 입력 해석', () => {
    const bounds = { min: 1, max: 1000 }

    test('범위 안의 정수만 받고 쉼표와 앞뒤 공백은 무시한다', () => {
        expect(parseLimitInput('30', bounds)).toBe(30)
        expect(parseLimitInput(' 1,000 ', bounds)).toBe(1000)
        expect(parseLimitInput('1', bounds)).toBe(1)
    })

    test('범위 밖이거나 정수가 아니면 null 이다', () => {
        for (const text of ['', '0', '1001', '-1', '1.5', '12a', '1e3', '9'.repeat(40)]) expect(parseLimitInput(text, bounds)).toBeNull()
    })
})
