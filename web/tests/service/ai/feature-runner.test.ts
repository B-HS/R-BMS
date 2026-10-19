import { describe, expect, spyOn, test } from 'bun:test'
import { z } from 'zod'
import type { AiFeatureRunInput } from '@server/service/domain/ai/feature-runner.service'
import { FAKE_USAGE, createFakeAdapter, createMemoryAiServices, testKekEnv, type FakeAdapterOptions } from '../../support/ai-memory-db'

const USER = 'user-alice'
const ANTHROPIC_KEY = 'sk-ant-api03-runner-secret-AAAA'
const OPENAI_KEY = 'sk-proj-runner-secret-BBBB'
const outputSchema = z.object({ title: z.string().max(20), keys: z.array(z.string()) })
const DIGEST = { items: [{ key: 'seg.s1.bp', value: 3 }] }

const consoleSpies = () =>
    [spyOn(console, 'error'), spyOn(console, 'log'), spyOn(console, 'warn')].map((spy) => spy.mockImplementation(() => undefined))

const logged = (spies: ReturnType<typeof consoleSpies>) => JSON.stringify(spies.flatMap((spy) => spy.mock.calls))

const setup = (generate?: FakeAdapterOptions['generate']) => {
    const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([ANTHROPIC_KEY]), generate })
    const openai = createFakeAdapter('openai', { validKeys: new Set([OPENAI_KEY]), generate })
    const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter, openai: openai.adapter }, env: () => testKekEnv('v1') })
    return { ...services, anthropic, openai }
}

const register = async (services: ReturnType<typeof setup>, provider: 'anthropic' | 'openai', apiKey: string, model: string | null) => {
    expect((await services.aiCredentialService.register({ userId: USER, provider, apiKey })).ok).toBe(true)
    if (model) await services.aiCredentialService.setModel({ userId: USER, provider, model })
}

const runOnce = (services: ReturnType<typeof setup>, overrides: Partial<AiFeatureRunInput<typeof outputSchema>> = {}) =>
    services.aiFeatureRunner.run({
        userId: USER,
        feature: 'query',
        subjectKey: 'q:abc',
        analysisVersion: 1,
        promptVersion: 1,
        digest: DIGEST,
        system: 'system prompt',
        schemaName: 'query_filters',
        schema: outputSchema,
        maxOutputTokens: 512,
        ...overrides,
    })

describe('AI 호출 감싸기', () => {
    test('키·모델이 없으면 공급자를 부르지 않는다', async () => {
        const services = setup()
        expect(await runOnce(services)).toEqual({ ok: false, reason: 'credential_not_found' })
        await register(services, 'anthropic', ANTHROPIC_KEY, null)
        expect(await runOnce(services)).toEqual({ ok: false, reason: 'model_not_selected', provider: 'anthropic' })
        expect(services.anthropic.generateCalls).toHaveLength(0)
    })

    test('성공하면 출력·사용량을 저장하고 같은 입력은 캐시로 돌려 공급자를 다시 부르지 않는다', async () => {
        const services = setup(() => ({ data: { title: '트릴 약점', keys: ['seg.s1.bp'] }, usage: FAKE_USAGE }))
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        const first = await runOnce(services)
        expect(first).toMatchObject({ ok: true, value: { cached: false, provider: 'anthropic', model: 'claude-test', usage: FAKE_USAGE } })
        expect(services.anthropic.generateCalls).toHaveLength(1)
        expect(services.anthropic.generateCalls[0]).toMatchObject({
            model: 'claude-test',
            input: JSON.stringify(DIGEST),
            schemaName: 'query_filters',
        })
        expect(services.memory.results.size).toBe(1)
        expect(services.memory.slots.size).toBe(0)
        const usage = await services.aiUsageService.summary({ userId: USER, limits: await services.aiSettingsService.resolveLimits(USER) })
        expect(usage.today).toEqual({ calls: 1, input_tokens: 120, output_tokens: 40, cached_input_tokens: 20 })

        const second = await runOnce(services)
        expect(second).toMatchObject({ ok: true, value: { cached: true, output: { title: '트릴 약점' } } })
        expect(services.anthropic.generateCalls).toHaveLength(1)
        const changed = await runOnce(services, { digest: { items: [] } })
        expect(changed).toMatchObject({ ok: true, value: { cached: false } })
        expect(services.anthropic.generateCalls).toHaveLength(2)
    })

    test('기본 공급자 설정을 따르고 없으면 먼저 등록한 키를 쓴다', async () => {
        const services = setup(() => ({ data: { title: 't', keys: [] }, usage: FAKE_USAGE }))
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        await register(services, 'openai', OPENAI_KEY, 'gpt-test')
        expect(await runOnce(services)).toMatchObject({ ok: true, value: { provider: 'anthropic' } })
        await services.aiSettingsService.putUserSetting({ userId: USER, input: { monthly_token_limit: null, default_provider: 'openai' } })
        expect(await runOnce(services)).toMatchObject({ ok: true, value: { provider: 'openai', model: 'gpt-test' } })
        expect(services.openai.seenKeys).toContain(OPENAI_KEY)
        expect(services.anthropic.generateCalls.every((call) => call.apiKey === ANTHROPIC_KEY)).toBe(true)
    })

    test('공급자 실패는 캐시하지 않고 사용량은 남기며 자리를 돌려준다', async () => {
        const services = setup(() => ({ failure: 'truncated', usage: FAKE_USAGE }))
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        const result = await runOnce(services)
        expect(result).toMatchObject({ ok: false, reason: 'provider', provider: 'anthropic', detail: { kind: 'truncated' } })
        expect(services.memory.results.size).toBe(0)
        expect(services.memory.slots.size).toBe(0)
        const usage = await services.aiUsageService.summary({ userId: USER, limits: await services.aiSettingsService.resolveLimits(USER) })
        expect(usage.today.calls).toBe(1)
        expect(usage.today.output_tokens).toBe(FAKE_USAGE.outputTokens)
    })

    test('서버 Zod 재검증·기능 검사를 통과하지 못하면 output_invalid 이고 저장하지 않는다', async () => {
        const services = setup(() => ({ data: { title: 'x', keys: ['missing'] }, usage: FAKE_USAGE }))
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        const refined = await runOnce(services, { refine: (data) => (data.keys.every((key) => key === 'seg.s1.bp') ? data : null) })
        expect(refined).toEqual({ ok: false, reason: 'output_invalid', provider: 'anthropic' })
        expect(services.memory.results.size).toBe(0)
    })

    test('공급자가 키를 거절하면 키 카드에 invalid_key 를 남긴다', async () => {
        const services = setup()
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        services.anthropic.adapter.generateStructured = async () => ({
            ok: false,
            usage: null,
            kind: 'invalid_key',
            requestId: null,
            httpStatus: 401,
            providerErrorType: 'authentication_error',
            retryAfterSeconds: null,
        })
        expect(await runOnce(services)).toMatchObject({ ok: false, reason: 'provider', detail: { kind: 'invalid_key' } })
        expect((await services.aiCredentialService.list(USER))[0].last_error_kind).toBe('invalid_key')
    })

    test('공급자가 403 으로 막으면 forbidden 이고 키를 거절된 키로 표시하지 않는다', async () => {
        const services = setup()
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        services.anthropic.adapter.generateStructured = async () => ({
            ok: false,
            usage: null,
            kind: 'forbidden',
            requestId: null,
            httpStatus: 403,
            providerErrorType: 'permission_error',
            retryAfterSeconds: null,
        })
        expect(await runOnce(services)).toMatchObject({ ok: false, reason: 'provider', detail: { kind: 'forbidden' } })
        expect((await services.aiCredentialService.list(USER))[0].last_error_kind).toBeNull()
    })

    test('공급자 성공 뒤 자리 해제가 실패해도 결과를 돌려준다(남은 자리는 보유 시간이 지나면 풀린다)', async () => {
        const services = setup(() => ({ data: { title: 't', keys: [] }, usage: FAKE_USAGE }))
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        services.memory.usageDb.releaseSlot = async () => Promise.reject(new Error('connection lost'))
        expect(await runOnce(services)).toMatchObject({ ok: true, value: { cached: false, output: { title: 't' } } })
    })

    test('결과 저장이 실패해도 출력을 돌려주고 로그에는 본문 없이 기능 이름만 남긴다', async () => {
        const spies = consoleSpies()
        try {
            const services = setup(() => ({ data: { title: '비밀 출력 본문', keys: ['seg.s1.bp'] }, usage: FAKE_USAGE }))
            await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
            services.memory.resultDb.upsert = async () => Promise.reject(new Error('Failed query: insert into ai_result params: 비밀 출력 본문'))
            const result = await runOnce(services)
            expect(result).toMatchObject({ ok: true, value: { cached: false, output: { title: '비밀 출력 본문' }, usage: FAKE_USAGE } })
            const text = logged(spies)
            expect(text).toContain('query')
            expect(text).not.toContain('비밀 출력 본문')
            expect(text).not.toContain('seg.s1.bp')
        } finally {
            for (const spy of spies) spy.mockRestore()
        }
    })

    test('어댑터가 예외를 던져도 동시 실행 자리를 돌려준다', async () => {
        const services = setup()
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        services.anthropic.adapter.generateStructured = async () => Promise.reject(new TypeError('network'))
        await expect(runOnce(services)).rejects.toThrow('network')
        expect(services.memory.slots.size).toBe(0)
    })

    test('하루 한도에 닿으면 공급자를 부르지 않고 usage_limit 이다', async () => {
        const services = setup(() => ({ data: { title: 't', keys: [] }, usage: FAKE_USAGE }))
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        await services.aiSettingsService.putOperatorLimits({
            userId: 'admin',
            input: { daily_calls: 1, concurrent_runs: 1, default_monthly_token_limit: null },
        })
        expect((await runOnce(services)).ok).toBe(true)
        expect(await runOnce(services, { subjectKey: 'q:other' })).toEqual({ ok: false, reason: 'usage_limit', limit: 'daily_calls' })
        expect(services.anthropic.generateCalls).toHaveLength(1)
    })

    test('결과·실패·로그 어디에도 키 평문이 없다', async () => {
        const spies = consoleSpies()
        try {
            const services = setup(() => ({ failure: 'bad_request', usage: null }))
            await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
            const failure = await runOnce(services)
            const listed = await services.aiCredentialService.list(USER)
            const stored = JSON.stringify([...services.memory.credentials.values()])
            for (const text of [JSON.stringify(failure), JSON.stringify(listed), stored, logged(spies)]) expect(text).not.toContain(ANTHROPIC_KEY)
            expect(listed[0].key_hint).toBe(ANTHROPIC_KEY.slice(-4))
        } finally {
            for (const spy of spies) spy.mockRestore()
        }
    })
})

describe('키 서비스', () => {
    test('공급자 확인에 실패한 키는 저장하지 않는다', async () => {
        const services = setup()
        const result = await services.aiCredentialService.register({ userId: USER, provider: 'anthropic', apiKey: 'sk-ant-wrong-key-000' })
        expect(result).toMatchObject({ ok: false, reason: 'provider', detail: { kind: 'invalid_key' } })
        expect(services.memory.credentials.size).toBe(0)
    })

    test('KEK 가 없으면 공급자를 부르기 전에 503 으로 멈춘다', async () => {
        const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([ANTHROPIC_KEY]) })
        const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => ({}) })
        await expect(services.aiCredentialService.register({ userId: USER, provider: 'anthropic', apiKey: ANTHROPIC_KEY })).rejects.toMatchObject({
            code: 'SERVICE_NOT_CONFIGURED',
            statusCode: 503,
        })
        expect(anthropic.seenKeys).toHaveLength(0)
    })

    test('KEK 회전 뒤 키를 쓰면 행이 새 버전으로 다시 감싸이고 계속 열린다', async () => {
        const env = { current: testKekEnv('v1') }
        const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([ANTHROPIC_KEY]) })
        const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => env.current })
        await services.aiCredentialService.register({ userId: USER, provider: 'anthropic', apiKey: ANTHROPIC_KEY })
        const before = [...services.memory.credentials.values()][0]
        expect(before.kekVersion).toBe('v1')
        env.current = testKekEnv('v2', ['v1', 'v2'])
        expect((await services.aiCredentialService.revalidate({ userId: USER, provider: 'anthropic' })).ok).toBe(true)
        const after = [...services.memory.credentials.values()][0]
        expect(after.kekVersion).toBe('v2')
        expect(after.ciphertext).toBe(before.ciphertext)
        env.current = testKekEnv('v2')
        expect((await services.aiCredentialService.listModels({ userId: USER, provider: 'anthropic' })).ok).toBe(true)
        expect(anthropic.seenKeys.every((key) => key === ANTHROPIC_KEY)).toBe(true)
    })

    test('KEK 회전 뒤 재포장 저장이 실패해도 키를 쓰는 호출은 계속된다', async () => {
        const env = { current: testKekEnv('v1') }
        const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([ANTHROPIC_KEY]) })
        const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => env.current })
        await services.aiCredentialService.register({ userId: USER, provider: 'anthropic', apiKey: ANTHROPIC_KEY })
        env.current = testKekEnv('v2', ['v1', 'v2'])
        services.memory.credentialDb.rewrapCredential = async () => Promise.reject(new Error('deadlock'))
        expect((await services.aiCredentialService.listModels({ userId: USER, provider: 'anthropic' })).ok).toBe(true)
        expect([...services.memory.credentials.values()][0].kekVersion).toBe('v1')
    })

    test('재포장은 읽었던 감싼 DEK 가 그대로일 때만 저장한다', async () => {
        const env = { current: testKekEnv('v1') }
        const anthropic = createFakeAdapter('anthropic', { validKeys: new Set([ANTHROPIC_KEY]) })
        const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => env.current })
        await services.aiCredentialService.register({ userId: USER, provider: 'anthropic', apiKey: ANTHROPIC_KEY })
        const calls: unknown[] = []
        const rewrap = services.memory.credentialDb.rewrapCredential
        services.memory.credentialDb.rewrapCredential = async (params) => {
            calls.push(params.previous)
            return rewrap(params)
        }
        const before = [...services.memory.credentials.values()][0]
        env.current = testKekEnv('v2', ['v1', 'v2'])
        await services.aiCredentialService.revalidate({ userId: USER, provider: 'anthropic' })
        expect(calls).toEqual([{ kekVersion: 'v1', wrappedDek: before.wrappedDek, dekIv: before.dekIv }])
    })

    test('다른 사용자 행으로 옮긴 암호문은 열리지 않는다', async () => {
        const services = setup()
        await register(services, 'anthropic', ANTHROPIC_KEY, 'claude-test')
        const row = services.memory.credentials.get(`${USER}|anthropic`)
        if (row) services.memory.credentials.set('user-bob|anthropic', { ...row, userId: 'user-bob' })
        expect(await services.aiCredentialService.listModels({ userId: 'user-bob', provider: 'anthropic' })).toEqual({
            ok: false,
            reason: 'credential_undecryptable',
            cause: 'tampered',
        })
    })
})
