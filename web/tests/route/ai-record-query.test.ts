import { beforeEach, describe, expect, spyOn, test } from 'bun:test'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAiRecordQueryRoutes } from '@server/route/ai'
import { createAiRecordQueryService, type AiRecordQueryServiceDb } from '@server/service/domain/ai/record-query.service'
import { FAKE_USAGE, createFakeAdapter, createMemoryAiServices, testKekEnv } from '../support/ai-memory-db'

const ALICE: AuthUser = { id: 'user-alice', loginId: 'alice', name: 'Alice', role: 'user' }
const API_KEY = 'sk-ant-api03-route-record-secret-K7L8'
const ORIGIN = 'http://localhost:3000'
const ASK_PATH = '/api/fe/me/ai/record-query'
const RUN_PATH = `${ASK_PATH}/run`
const PLAN = { filters: [{ field: 'bp', op: 'lte', value: '5' }], sort_field: 'played_at', sort_dir: 'desc', limit: 10 }

const createRoutes = () => {
    const anthropic = createFakeAdapter('anthropic', {
        validKeys: new Set([API_KEY]),
        generate: () => ({ data: { ...PLAN, explanation: 'BP 5 이하' }, usage: FAKE_USAGE }),
    })
    const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => testKekEnv('v1') })
    const seenUserIds: string[] = []
    const db: AiRecordQueryServiceDb = {
        listScoreRecords: async ({ userId }) => {
            seenUserIds.push(userId)
            return []
        },
        listTableBestRecords: async ({ userId }) => {
            seenUserIds.push(userId)
            return []
        },
        listPlayedMd5s: async ({ userId }) => {
            seenUserIds.push(userId)
            return []
        },
    }
    const aiRecordQueryService = createAiRecordQueryService({
        runner: services.aiFeatureRunner,
        db,
        loadTables: () => [],
        now: () => Date.UTC(2026, 9, 19),
    })
    const routes = createAiRecordQueryRoutes(
        () => ({
            authService: { resolveSessionUser: async (request) => (request.headers.get('x-session-user') === ALICE.loginId ? ALICE : null) },
            aiRecordQueryService,
        }),
        async () => undefined,
    )
    return { routes, services, anthropic, seenUserIds }
}

const call = (path: string, init: { user?: AuthUser; bearer?: string; body?: unknown } = {}) =>
    new Request(`${ORIGIN}${path}`, {
        method: 'POST',
        headers: {
            'content-type': 'application/json',
            ...(init.user ? { 'x-session-user': init.user.loginId } : {}),
            ...(init.bearer ? { authorization: `Bearer ${init.bearer}` } : {}),
        },
        body: init.body === undefined ? undefined : JSON.stringify(init.body),
    })

beforeEach(() => resetRateLimits())

describe('기록 질문 API', () => {
    test('세션이 없거나 API 토큰(Bearer)만 있으면 401 이고 공급자·DB 를 부르지 않는다', async () => {
        const { routes, anthropic, seenUserIds } = createRoutes()
        for (const init of [{}, { bearer: 'rbms_token' }]) {
            expect((await routes.ask(call(ASK_PATH, { ...init, body: { question: 'q' } }), undefined)).status).toBe(401)
            expect((await routes.run(call(RUN_PATH, { ...init, body: PLAN }), undefined)).status).toBe(401)
        }
        expect(anthropic.generateCalls).toHaveLength(0)
        expect(seenUserIds).toHaveLength(0)
    })

    test('질문은 비어 있거나 200자를 넘으면 400 이다', async () => {
        const { routes, anthropic } = createRoutes()
        for (const question of ['   ', 'x'.repeat(201)]) {
            const response = await routes.ask(call(ASK_PATH, { user: ALICE, body: { question } }), undefined)
            expect(response.status).toBe(400)
        }
        expect(anthropic.generateCalls).toHaveLength(0)
    })

    test('키가 없으면 AI_CREDENTIAL_NOT_FOUND 이고, 등록 뒤에는 결과를 비공개 응답으로 주며 키를 담지 않는다', async () => {
        const errors = spyOn(console, 'error').mockImplementation(() => undefined)
        try {
            const { routes, services } = createRoutes()
            const missing = await routes.ask(call(ASK_PATH, { user: ALICE, body: { question: 'BP 5 이하' } }), undefined)
            expect(missing.status).toBe(404)
            expect((await missing.json()).error.code).toBe('AI_CREDENTIAL_NOT_FOUND')
            await services.aiCredentialService.register({ userId: ALICE.id, provider: 'anthropic', apiKey: API_KEY })
            await services.aiCredentialService.setModel({ userId: ALICE.id, provider: 'anthropic', model: 'claude-test' })
            const response = await routes.ask(call(ASK_PATH, { user: ALICE, body: { question: 'BP 5 이하' } }), undefined)
            expect(response.status).toBe(200)
            expect(response.headers.get('cache-control')).toBe('private, no-store')
            const text = await response.text()
            expect(text).not.toContain(API_KEY)
            expect(JSON.parse(text).data).toMatchObject({ plan: { explanation: 'BP 5 이하' }, issues: [], result: { source: 'scores', rows: [] } })
            expect(JSON.stringify(errors.mock.calls)).not.toContain(API_KEY)
        } finally {
            errors.mockRestore()
        }
    })

    test('다시 실행은 AI 를 부르지 않고, 본문에 다른 사용자 id 를 넣어도 세션 사용자로만 읽는다', async () => {
        const { routes, anthropic, seenUserIds } = createRoutes()
        const response = await routes.run(call(RUN_PATH, { user: ALICE, body: { ...PLAN, user_id: 'user-bob', userId: 'user-bob' } }), undefined)
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        expect((await response.json()).data).toMatchObject({ ai: null, plan: { explanation: null } })
        expect(anthropic.generateCalls).toHaveLength(0)
        expect(seenUserIds).toEqual([ALICE.id])
    })

    test('허용되지 않은 항목 이름은 400 이고 검사에 실패한 값은 200 과 함께 틀린 필터를 알려 준다', async () => {
        const { routes, seenUserIds } = createRoutes()
        const unknown = await routes.run(
            call(RUN_PATH, { user: ALICE, body: { ...PLAN, filters: [{ field: 'user', op: 'eq', value: 'bob' }] } }),
            undefined,
        )
        expect(unknown.status).toBe(400)
        const invalid = await routes.run(
            call(RUN_PATH, { user: ALICE, body: { ...PLAN, filters: [{ field: 'bp', op: 'lte', value: '-1' }] } }),
            undefined,
        )
        expect(invalid.status).toBe(200)
        expect((await invalid.json()).data).toMatchObject({ issues: [{ index: 0, reason: 'value_invalid' }], result: null })
        expect(seenUserIds).toHaveLength(0)
    })
})
