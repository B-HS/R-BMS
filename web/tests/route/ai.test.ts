import { beforeEach, describe, expect, spyOn, test } from 'bun:test'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAiRoutes, toAiAppError } from '@server/route/ai'
import { createFailureDetail } from '@server/service/shared/ai-provider/http'
import { createFakeAdapter, createMemoryAiServices, testKekEnv } from '../support/ai-memory-db'

const ALICE: AuthUser = { id: 'user-alice', loginId: 'alice', name: 'Alice', role: 'user' }
const ADMIN: AuthUser = { id: 'user-admin', loginId: 'admin', name: 'Admin', role: 'admin' }
const USERS = new Map([ALICE, ADMIN].map((user) => [user.loginId, user]))
const API_KEY = 'sk-ant-api03-route-secret-Z9Y8'
const ORIGIN = 'http://localhost:3000'
const KEY_CHECKS_PER_MINUTE = 5

const createRoutes = (env: () => Record<string, string> = () => testKekEnv('v1')) => {
    const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([API_KEY]) })
    const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env })
    const routes = createAiRoutes(
        () => ({
            authService: { resolveSessionUser: async (request) => USERS.get(request.headers.get('x-session-user') ?? '') ?? null },
            ...services,
        }),
        async () => undefined,
    )
    return { routes, services, anthropic }
}

const call = (method: string, path: string, init: { user?: AuthUser; bearer?: string; body?: unknown } = {}) =>
    new Request(`${ORIGIN}${path}`, {
        method,
        headers: {
            'content-type': 'application/json',
            ...(init.user ? { 'x-session-user': init.user.loginId } : {}),
            ...(init.bearer ? { authorization: `Bearer ${init.bearer}` } : {}),
        },
        body: init.body === undefined ? undefined : JSON.stringify(init.body),
    })

const providerContext = (provider: string) => ({ params: Promise.resolve({ provider }) })

const PATH = '/api/fe/me/ai/credentials/anthropic'

const putKey = (routes: ReturnType<typeof createRoutes>['routes'], apiKey: string, user: AuthUser = ALICE) =>
    routes.putCredential(call('PUT', PATH, { user, body: { api_key: apiKey } }), providerContext('anthropic'))

beforeEach(() => resetRateLimits())

describe('AI 키 API 의 인증', () => {
    test('세션이 없거나 API 토큰(Bearer)만 있으면 모든 경로가 401 이다', async () => {
        const { routes, anthropic } = createRoutes()
        for (const init of [{}, { bearer: 'rbms_token' }]) {
            const responses = await Promise.all([
                routes.listCredentials(call('GET', '/api/fe/me/ai/credentials', init), undefined),
                routes.putCredential(call('PUT', PATH, { ...init, body: { api_key: API_KEY } }), providerContext('anthropic')),
                routes.patchCredential(call('PATCH', PATH, { ...init, body: { model: 'm' } }), providerContext('anthropic')),
                routes.deleteCredential(call('DELETE', PATH, init), providerContext('anthropic')),
                routes.validateCredential(call('POST', `${PATH}/validate`, init), providerContext('anthropic')),
                routes.listModels(call('GET', `${PATH}/models`, init), providerContext('anthropic')),
                routes.usage(call('GET', '/api/fe/me/ai/usage', init), undefined),
                routes.getSettings(call('GET', '/api/fe/me/ai/settings', init), undefined),
                routes.putSettings(call('PUT', '/api/fe/me/ai/settings', { ...init, body: {} }), undefined),
                routes.getLimits(call('GET', '/api/admin/ai/limits', init), undefined),
                routes.putLimits(call('PUT', '/api/admin/ai/limits', { ...init, body: {} }), undefined),
            ])
            for (const response of responses) expect(response.status).toBe(401)
        }
        expect(anthropic.seenKeys).toHaveLength(0)
    })
})

describe('키 등록·조회·삭제', () => {
    test('등록 응답과 목록에는 끝 4자만 있고 평문은 응답·로그 어디에도 없다', async () => {
        const errors = spyOn(console, 'error').mockImplementation(() => undefined)
        try {
            const { routes } = createRoutes()
            const response = await putKey(routes, API_KEY)
            expect(response.status).toBe(200)
            expect(response.headers.get('cache-control')).toBe('private, no-store')
            const registered = await response.text()
            expect(JSON.parse(registered).data).toMatchObject({ provider: 'anthropic', key_hint: 'Z9Y8', model: null, last_error_kind: null })
            const listed = await (await routes.listCredentials(call('GET', '/api/fe/me/ai/credentials', { user: ALICE }), undefined)).text()
            for (const text of [registered, listed, JSON.stringify(errors.mock.calls)]) expect(text).not.toContain(API_KEY)
            expect(Object.keys(JSON.parse(listed).data[0]).toSorted()).toEqual(
                ['created_at', 'key_hint', 'last_error_kind', 'last_validated_at', 'model', 'provider'].toSorted(),
            )
        } finally {
            errors.mockRestore()
        }
    })

    test('공급자가 거절한 키는 400 AI_CREDENTIAL_INVALID 이고 저장되지 않으며 오류에도 키가 없다', async () => {
        const { routes, services } = createRoutes()
        const wrongKey = 'sk-ant-api03-wrong-secret-0000'
        const response = await putKey(routes, wrongKey)
        expect(response.status).toBe(400)
        const text = await response.text()
        expect(JSON.parse(text).error.code).toBe('AI_CREDENTIAL_INVALID')
        expect(text).not.toContain(wrongKey)
        expect(services.memory.credentials.size).toBe(0)
    })

    test('형식이 틀린 키는 공급자를 부르지 않고 400 이며 오류에 키가 없다', async () => {
        const { routes, anthropic } = createRoutes()
        for (const apiKey of ['short', 'sk-ant has spaces inside', 'x'.repeat(513)]) {
            const response = await putKey(routes, apiKey)
            expect(response.status).toBe(400)
            expect(await response.text()).not.toContain(apiKey)
        }
        expect(anthropic.seenKeys).toHaveLength(0)
    })

    test('알 수 없는 공급자는 400 이다', async () => {
        const { routes } = createRoutes()
        const response = await routes.putCredential(
            call('PUT', '/api/fe/me/ai/credentials/gemini', { user: ALICE, body: { api_key: API_KEY } }),
            providerContext('gemini'),
        )
        expect(response.status).toBe(400)
        expect((await response.json()).error.details.field).toBe('provider')
    })

    test('모델을 고르고 모델 목록·다시 검증·삭제가 된다', async () => {
        const { routes } = createRoutes()
        await putKey(routes, API_KEY)
        const models = await routes.listModels(call('GET', `${PATH}/models`, { user: ALICE }), providerContext('anthropic'))
        expect((await models.json()).data).toEqual([{ id: 'anthropic-model', label: 'anthropic model', structured: true }])
        const patched = await routes.patchCredential(
            call('PATCH', PATH, { user: ALICE, body: { model: 'anthropic-model' } }),
            providerContext('anthropic'),
        )
        expect((await patched.json()).data.model).toBe('anthropic-model')
        const validated = await routes.validateCredential(call('POST', `${PATH}/validate`, { user: ALICE }), providerContext('anthropic'))
        expect(validated.status).toBe(200)
        const removed = await routes.deleteCredential(call('DELETE', PATH, { user: ALICE }), providerContext('anthropic'))
        expect(removed.status).toBe(204)
        const again = await routes.deleteCredential(call('DELETE', PATH, { user: ALICE }), providerContext('anthropic'))
        expect(again.status).toBe(404)
        expect((await again.json()).error.code).toBe('AI_CREDENTIAL_NOT_FOUND')
    })

    test('KEK 환경 변수가 없으면 503 이고 공급자를 부르지 않는다', async () => {
        const { routes, anthropic } = createRoutes(() => ({}))
        const response = await putKey(routes, API_KEY)
        expect(response.status).toBe(503)
        expect((await response.json()).error.code).toBe('SERVICE_NOT_CONFIGURED')
        expect(anthropic.seenKeys).toHaveLength(0)
    })

    test('키 등록·검증은 사용자당 분당 5회까지다', async () => {
        const { routes } = createRoutes()
        const statuses = []
        for (let index = 0; index <= KEY_CHECKS_PER_MINUTE; index += 1) statuses.push((await putKey(routes, API_KEY)).status)
        expect(statuses.slice(0, KEY_CHECKS_PER_MINUTE).every((status) => status === 200)).toBe(true)
        expect(statuses.at(-1)).toBe(429)
    })
})

describe('사용량과 상한', () => {
    test('사용량 요약은 기본 한도와 사용자가 정한 월 토큰 상한을 보인다', async () => {
        const { routes } = createRoutes()
        const before = await (await routes.usage(call('GET', '/api/fe/me/ai/usage', { user: ALICE }), undefined)).json()
        expect(before.data.limits).toEqual({ daily_calls: 30, concurrent_runs: 1, monthly_token_limit: null })
        const put = await routes.putSettings(
            call('PUT', '/api/fe/me/ai/settings', { user: ALICE, body: { monthly_token_limit: 200000, default_provider: 'anthropic' } }),
            undefined,
        )
        expect((await put.json()).data).toMatchObject({ monthly_token_limit: 200000, default_provider: 'anthropic' })
        const after = await (await routes.usage(call('GET', '/api/fe/me/ai/usage', { user: ALICE }), undefined)).json()
        expect(after.data.limits.monthly_token_limit).toBe(200000)
        const invalid = await routes.putSettings(
            call('PUT', '/api/fe/me/ai/settings', { user: ALICE, body: { monthly_token_limit: -1, default_provider: null } }),
            undefined,
        )
        expect(invalid.status).toBe(400)
    })
})

describe('운영자 한도', () => {
    const limits = { daily_calls: 60, concurrent_runs: 2, default_monthly_token_limit: 500000 }

    test('관리자가 아니면 읽기·쓰기 모두 403 이다', async () => {
        const { routes } = createRoutes()
        expect((await routes.getLimits(call('GET', '/api/admin/ai/limits', { user: ALICE }), undefined)).status).toBe(403)
        expect((await routes.putLimits(call('PUT', '/api/admin/ai/limits', { user: ALICE, body: limits }), undefined)).status).toBe(403)
    })

    test('관리자가 바꾼 값이 모든 사용자의 한도가 된다', async () => {
        const { routes } = createRoutes()
        const defaults = await (await routes.getLimits(call('GET', '/api/admin/ai/limits', { user: ADMIN }), undefined)).json()
        expect(defaults.data).toEqual({ daily_calls: 30, concurrent_runs: 1, default_monthly_token_limit: null, updated_at: null })
        const put = await routes.putLimits(call('PUT', '/api/admin/ai/limits', { user: ADMIN, body: limits }), undefined)
        expect(put.status).toBe(200)
        const usage = await (await routes.usage(call('GET', '/api/fe/me/ai/usage', { user: ALICE }), undefined)).json()
        expect(usage.data.limits).toEqual({ daily_calls: 60, concurrent_runs: 2, monthly_token_limit: 500000 })
    })

    test('범위를 벗어난 운영자 값은 400 이다', async () => {
        const { routes } = createRoutes()
        for (const body of [
            { ...limits, daily_calls: 0 },
            { ...limits, concurrent_runs: 6 },
            { ...limits, default_monthly_token_limit: 0 },
        ]) {
            expect((await routes.putLimits(call('PUT', '/api/admin/ai/limits', { user: ADMIN, body }), undefined)).status).toBe(400)
        }
    })
})

describe('AI 실패를 API 오류로 바꾸기', () => {
    test('실패 종류마다 기획 6.5 의 코드와 상태가 되고 bad_request 만 서버 로그에 남긴다', () => {
        const errors = spyOn(console, 'error').mockImplementation(() => undefined)
        try {
            const expected = [
                ['invalid_key', 'AI_CREDENTIAL_INVALID', 400],
                ['forbidden', 'AI_PROVIDER_FORBIDDEN', 403],
                ['model_unavailable', 'AI_MODEL_UNAVAILABLE', 422],
                ['quota', 'AI_PROVIDER_QUOTA', 402],
                ['rate_limited', 'AI_PROVIDER_RATE_LIMITED', 429],
                ['unavailable', 'AI_PROVIDER_UNAVAILABLE', 502],
                ['refused', 'AI_PROVIDER_REFUSED', 422],
                ['truncated', 'AI_OUTPUT_TRUNCATED', 502],
                ['invalid_output', 'AI_OUTPUT_INVALID', 502],
                ['bad_request', 'AI_PROVIDER_BAD_REQUEST', 502],
            ] as const
            for (const [kind, code, status] of expected) {
                const error = toAiAppError({ reason: 'provider', provider: 'anthropic', detail: createFailureDetail(kind, { requestId: 'req_1' }) })
                expect([error.code, error.statusCode, error.details?.reason]).toEqual([code, status, kind])
            }
            expect(errors).toHaveBeenCalledTimes(1)
            expect(toAiAppError({ reason: 'usage_limit', limit: 'concurrent_runs' })).toMatchObject({ code: 'AI_USAGE_LIMIT', statusCode: 429 })
            expect(toAiAppError({ reason: 'credential_undecryptable', cause: 'unknown_kek' }).statusCode).toBe(503)
            expect(toAiAppError({ reason: 'model_not_selected', provider: 'openai' }).details).toEqual({
                field: 'model',
                reason: 'model_not_selected',
            })
        } finally {
            errors.mockRestore()
        }
    })

    test('INTERNAL_ERROR 는 우리 쪽 스키마를 공급자 형식으로 바꾸지 못한 경우뿐이다', () => {
        const errors = spyOn(console, 'error').mockImplementation(() => undefined)
        try {
            const unsupported = toAiAppError({
                reason: 'provider',
                provider: 'openai',
                detail: createFailureDetail('bad_request', { providerErrorType: 'schema_unsupported' }),
            })
            expect([unsupported.code, unsupported.statusCode]).toEqual(['INTERNAL_ERROR', 500])
            expect(errors).toHaveBeenCalledTimes(1)
        } finally {
            errors.mockRestore()
        }
    })
})

describe('공급자가 403 으로 막은 키', () => {
    const forbidden = createFailureDetail('forbidden', { httpStatus: 403, providerErrorType: 'permission_error' })

    test('키는 인증된 것으로 보고 경고(forbidden)와 함께 저장한다', async () => {
        const { routes, services, anthropic } = createRoutes()
        anthropic.adapter.validateKey = async () => ({ ok: false, ...forbidden })
        const response = await putKey(routes, API_KEY)
        expect(response.status).toBe(200)
        expect((await response.json()).data).toMatchObject({ key_hint: 'Z9Y8', last_error_kind: 'forbidden' })
        expect(services.memory.credentials.size).toBe(1)
    })

    test('다시 검증에서 403 이면 경고만 남기고 성공으로 돌려준다', async () => {
        const { routes, anthropic } = createRoutes()
        await putKey(routes, API_KEY)
        anthropic.adapter.validateKey = async () => ({ ok: false, ...forbidden })
        const response = await routes.validateCredential(call('POST', `${PATH}/validate`, { user: ALICE }), providerContext('anthropic'))
        expect(response.status).toBe(200)
        expect((await response.json()).data.last_error_kind).toBe('forbidden')
    })

    test('모델 목록이 403 이면 AI_PROVIDER_FORBIDDEN 이고 키를 거절된 키로 표시하지 않으며 모델 id 를 직접 고를 수 있다', async () => {
        const { routes, services, anthropic } = createRoutes()
        await putKey(routes, API_KEY)
        anthropic.adapter.listModels = async () => ({ ok: false, ...forbidden })
        const models = await routes.listModels(call('GET', `${PATH}/models`, { user: ALICE }), providerContext('anthropic'))
        expect(models.status).toBe(403)
        const body = await models.json()
        expect(body.error).toMatchObject({ code: 'AI_PROVIDER_FORBIDDEN', message: expect.stringContaining('키 권한 또는 모델 접근을 확인하세요') })
        expect((await services.aiCredentialService.list(ALICE.id))[0].last_error_kind).toBeNull()
        const patched = await routes.patchCredential(
            call('PATCH', PATH, { user: ALICE, body: { model: 'claude-typed-by-hand-1' } }),
            providerContext('anthropic'),
        )
        expect((await patched.json()).data.model).toBe('claude-typed-by-hand-1')
    })
})

describe('AI 라우트의 처리되지 않은 오류 로그', () => {
    test('키 쓰기의 DB 오류는 500 이고 로그·응답에 암호문이나 쿼리 값이 남지 않는다', async () => {
        const errors = spyOn(console, 'error').mockImplementation(() => undefined)
        try {
            const { routes, services } = createRoutes()
            const leaked: string[] = []
            services.memory.credentialDb.upsertCredential = async ({ sealed }) => {
                leaked.push(sealed.ciphertext, sealed.wrappedDek)
                throw new Error(`Failed query: insert into ai_credential\nparams: ${sealed.ciphertext},${sealed.wrappedDek}`)
            }
            const response = await putKey(routes, API_KEY)
            expect(response.status).toBe(500)
            const text = await response.text()
            const logged = JSON.stringify(errors.mock.calls.map((args) => args.map((arg) => (arg instanceof Error ? arg.message : arg))))
            expect(errors).toHaveBeenCalled()
            for (const secret of [...leaked, API_KEY]) {
                expect(text).not.toContain(secret)
                expect(logged).not.toContain(secret)
            }
        } finally {
            errors.mockRestore()
        }
    })
})
