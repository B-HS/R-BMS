import { beforeEach, describe, expect, test } from 'bun:test'
import { weaknessProfileViewSchema } from '@server/dto/ai-play'
import type { AuthUser } from '@server/lib/auth-context'
import { RATE_LIMIT, resetRateLimits } from '@server/lib/with-rate-limit'
import { createAiPlayFeatureRoutes, type AiPlayFeatureRouteDeps } from '@server/route/ai'
import { createWeaknessProfileService } from '@server/service/domain/ai/weakness-profile.service'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import { createMemoryProfileDb } from '../support/ai-memory-db'
import { TRACE_USER_ID, createMemoryTraceDb, createMemoryTraceService, readContractTrace, scoreRowFor } from '../support/trace-fixture'

const NORMAL = readContractTrace('trace-normal.json')
const ALICE: AuthUser = { id: TRACE_USER_ID, loginId: 'alice', name: 'Alice', role: 'user' }
const BOB: AuthUser = { id: 'usr_bob', loginId: 'bob', name: 'Bob', role: 'user' }
const NOW = 1_791_970_080_000
const URL_OF_PROFILE = 'http://localhost:3000/api/fe/me/ai/profile'

const unused = { view: async () => Promise.reject(new Error('not called')), interpret: async () => Promise.reject(new Error('not called')) }

const createRoutes = async () => {
    const memory = createMemoryTraceDb([scoreRowFor(NORMAL, countTraceJudgements(NORMAL))])
    const { service } = createMemoryTraceService(memory)
    await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
    const profileDb = createMemoryProfileDb()
    const users = [ALICE, BOB]
    const deps: AiPlayFeatureRouteDeps = {
        authService: { resolveSessionUser: async (request) => users.find((user) => user.loginId === request.headers.get('x-session-user')) ?? null },
        aiPlayReportService: unused,
        aiReplayCoachService: unused,
        aiPlayCompareService: unused,
        aiGaugeAdviceService: unused,
        weaknessProfileService: createWeaknessProfileService({ db: profileDb.db, plays: service, now: () => NOW }),
    }
    return {
        routes: createAiPlayFeatureRoutes(
            () => deps,
            async () => undefined,
            async () => 'ko' as const,
        ),
        profileDb,
    }
}

const request = (init: { user?: AuthUser; bearer?: string } = {}) =>
    new Request(URL_OF_PROFILE, {
        headers: {
            ...(init.user ? { 'x-session-user': init.user.loginId } : {}),
            ...(init.bearer ? { authorization: `Bearer ${init.bearer}` } : {}),
        },
    })

beforeEach(() => resetRateLimits())

describe('GET /api/fe/me/ai/profile', () => {
    test('세션이 없거나 API 토큰만 있으면 401 이다', async () => {
        const { routes, profileDb } = await createRoutes()
        expect((await routes.profileView(request(), undefined)).status).toBe(401)
        expect((await routes.profileView(request({ bearer: 'rbms_token' }), undefined)).status).toBe(401)
        expect(profileDb.saves).toEqual([])
    })

    test('세션 사용자의 프로필만 private, no-store 로 답하고 응답은 프로필 스키마를 지킨다', async () => {
        const { routes, profileDb } = await createRoutes()
        const response = await routes.profileView(request({ user: ALICE }), undefined)
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        const body = await response.json()
        expect(weaknessProfileViewSchema.safeParse(body.data).success).toBe(true)
        expect(body.data).toMatchObject({ window: { plays: 1, stale: 0 }, notice: { need: 'traced_plays' } })
        const other = await (await routes.profileView(request({ user: BOB }), undefined)).json()
        expect(other.data.window.plays).toBe(0)
        expect(profileDb.saves).toEqual([ALICE.id, BOB.id])
    })

    test('분당 한도를 넘으면 429 이다', async () => {
        const { routes } = await createRoutes()
        for (let index = 0; index < RATE_LIMIT.TABLE_RECORDS.limit; index += 1) {
            expect((await routes.profileView(request({ user: ALICE }), undefined)).status).toBe(200)
        }
        expect((await routes.profileView(request({ user: ALICE }), undefined)).status).toBe(429)
    })
})
