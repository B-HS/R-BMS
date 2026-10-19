import { describe, expect, test } from 'bun:test'
import { AI_DEFAULT_LIMITS } from '@server/dto/ai'
import { AI_RUN_HOLD_MS, createAiUsageService, utcDayKey, utcMonthKey } from '@server/service/domain/ai/usage.service'
import { createAiSettingsService } from '@server/service/domain/ai/settings.service'
import { createMemoryAiDb } from '../../support/ai-memory-db'

const USER = 'user-alice'
const NOON = Date.UTC(2026, 9, 19, 12)
const DEFAULT_LIMITS = { dailyCalls: 3, concurrentRuns: 1, monthlyTokenLimit: null }

const setup = (start = NOON) => {
    const memory = createMemoryAiDb()
    const clock = { now: start }
    let counter = 0
    const service = createAiUsageService({ db: memory.usageDb, now: () => clock.now, newRunId: () => `run_${++counter}` })
    return { memory, clock, service }
}

describe('UTC 날짜 키', () => {
    test('하루와 달은 UTC 기준이다', () => {
        expect(utcDayKey(Date.UTC(2026, 9, 19, 23, 59))).toBe('2026-10-19')
        expect(utcDayKey(Date.UTC(2026, 9, 20, 0, 0))).toBe('2026-10-20')
        expect(utcMonthKey(NOON)).toBe('2026-10')
    })
})

describe('하루 호출 수', () => {
    test('한도까지만 잡히고 그다음은 daily_calls 로 거절하며 잡았던 동시 실행 자리를 돌려준다', async () => {
        const { service, memory } = setup()
        for (let index = 0; index < DEFAULT_LIMITS.dailyCalls; index += 1) {
            const reserved = await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })
            expect(reserved.ok).toBe(true)
            if (reserved.ok) await service.release(reserved.hold)
        }
        expect(await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).toEqual({ ok: false, limit: 'daily_calls' })
        expect(memory.slots.size).toBe(0)
    })

    test('UTC 날짜가 바뀌면 다시 셀 수 있다', async () => {
        const { service, clock } = setup(Date.UTC(2026, 9, 19, 23, 59))
        const limits = { ...DEFAULT_LIMITS, dailyCalls: 1 }
        const first = await service.reserve({ userId: USER, limits })
        if (first.ok) await service.release(first.hold)
        expect((await service.reserve({ userId: USER, limits })).ok).toBe(false)
        clock.now = Date.UTC(2026, 9, 20, 0, 1)
        expect((await service.reserve({ userId: USER, limits })).ok).toBe(true)
    })
})

describe('동시 실행', () => {
    test('자리를 잡은 뒤 하루 호출 수를 세다 오류가 나면 자리를 돌려주고 그 오류를 다시 던진다', async () => {
        const { service, memory } = setup()
        memory.usageDb.reserveDailyCall = async () => Promise.reject(new Error('lock wait timeout'))
        await expect(service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).rejects.toThrow('lock wait timeout')
        expect(memory.slots.size).toBe(0)
    })

    test('한 건이 잡고 있으면 둘째는 concurrent_runs 이고 하루 호출 수를 쓰지 않는다', async () => {
        const { service, memory } = setup()
        const first = await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })
        expect(first.ok).toBe(true)
        expect(await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).toEqual({ ok: false, limit: 'concurrent_runs' })
        expect((await service.summary({ userId: USER, limits: DEFAULT_LIMITS })).today.calls).toBe(1)
        if (first.ok) await service.release(first.hold)
        expect(memory.slots.size).toBe(0)
        expect((await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).ok).toBe(true)
    })

    test('운영자가 동시 실행을 2 로 올리면 두 건까지 잡힌다', async () => {
        const { service } = setup()
        const limits = { ...DEFAULT_LIMITS, concurrentRuns: 2 }
        expect((await service.reserve({ userId: USER, limits })).ok).toBe(true)
        expect((await service.reserve({ userId: USER, limits })).ok).toBe(true)
        expect(await service.reserve({ userId: USER, limits })).toEqual({ ok: false, limit: 'concurrent_runs' })
    })

    test('풀지 못한 자리는 보유 시간이 지나면 다른 실행이 가져간다', async () => {
        const { service, clock } = setup()
        expect((await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).ok).toBe(true)
        clock.now += AI_RUN_HOLD_MS - 1
        expect((await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).ok).toBe(false)
        clock.now += 1
        expect((await service.reserve({ userId: USER, limits: DEFAULT_LIMITS })).ok).toBe(true)
    })

    test('만료 뒤 늦게 끝난 옛 실행의 해제는 새 실행의 자리를 풀지 않는다', async () => {
        const { service, clock, memory } = setup()
        const stale = await service.reserve({ userId: USER, limits: { ...DEFAULT_LIMITS, dailyCalls: 5 } })
        clock.now += AI_RUN_HOLD_MS
        const fresh = await service.reserve({ userId: USER, limits: { ...DEFAULT_LIMITS, dailyCalls: 5 } })
        expect(fresh.ok).toBe(true)
        if (stale.ok) await service.release(stale.hold)
        expect(memory.slots.size).toBe(1)
    })
})

describe('월 토큰 상한', () => {
    test('이번 달 입력+출력 토큰이 상한에 닿으면 monthly_tokens 로 거절한다', async () => {
        const { service } = setup()
        const limits = { ...DEFAULT_LIMITS, dailyCalls: 10, monthlyTokenLimit: 1000 }
        const first = await service.reserve({ userId: USER, limits })
        expect(first.ok).toBe(true)
        if (!first.ok) return
        await service.record({ hold: first.hold, usage: { inputTokens: 700, outputTokens: 300, cachedInputTokens: 100 } })
        await service.release(first.hold)
        expect(await service.reserve({ userId: USER, limits })).toEqual({ ok: false, limit: 'monthly_tokens' })
        expect((await service.reserve({ userId: USER, limits: { ...limits, monthlyTokenLimit: 1001 } })).ok).toBe(true)
    })

    test('지난달 사용량은 이번 달 상한에 들어가지 않고 요약은 오늘과 이번 달을 따로 보인다', async () => {
        const { service, clock } = setup(Date.UTC(2026, 8, 30, 12))
        const limits = { ...DEFAULT_LIMITS, monthlyTokenLimit: 100 }
        const september = await service.reserve({ userId: USER, limits })
        expect(september.ok).toBe(true)
        if (!september.ok) return
        await service.record({ hold: september.hold, usage: { inputTokens: 500, outputTokens: 0, cachedInputTokens: 0 } })
        await service.release(september.hold)
        clock.now = NOON
        expect((await service.reserve({ userId: USER, limits })).ok).toBe(true)
        const summary = await service.summary({ userId: USER, limits })
        expect(summary).toMatchObject({ day: '2026-10-19', month: '2026-10', this_month: { calls: 1, input_tokens: 0 } })
        expect(summary.limits).toEqual({ daily_calls: 3, concurrent_runs: 1, monthly_token_limit: 100 })
    })
})

describe('운영자 한도와 사용자 상한', () => {
    test('행이 없으면 기본값(하루 30회, 동시 1건, 월 상한 없음)이다', async () => {
        const memory = createMemoryAiDb()
        const settings = createAiSettingsService({ db: memory.settingsDb, now: () => NOON })
        expect(await settings.resolveLimits(USER)).toEqual({
            dailyCalls: AI_DEFAULT_LIMITS.DAILY_CALLS,
            concurrentRuns: AI_DEFAULT_LIMITS.CONCURRENT_RUNS,
            monthlyTokenLimit: null,
        })
        expect(AI_DEFAULT_LIMITS).toMatchObject({ DAILY_CALLS: 30, CONCURRENT_RUNS: 1 })
    })

    test('운영자 값이 기본값을 대신하고 사용자의 월 상한이 운영자 기본 상한보다 먼저다', async () => {
        const memory = createMemoryAiDb()
        const settings = createAiSettingsService({ db: memory.settingsDb, now: () => NOON })
        await settings.putOperatorLimits({ userId: 'admin', input: { daily_calls: 50, concurrent_runs: 2, default_monthly_token_limit: 9000 } })
        expect(await settings.resolveLimits(USER)).toEqual({ dailyCalls: 50, concurrentRuns: 2, monthlyTokenLimit: 9000 })
        await settings.putUserSetting({ userId: USER, input: { monthly_token_limit: 500, default_provider: null } })
        expect(await settings.resolveLimits(USER)).toEqual({ dailyCalls: 50, concurrentRuns: 2, monthlyTokenLimit: 500 })
        expect(await settings.resolveLimits('user-bob')).toEqual({ dailyCalls: 50, concurrentRuns: 2, monthlyTokenLimit: 9000 })
    })
})
