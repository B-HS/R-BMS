import { beforeEach, describe, expect, test } from 'bun:test'
import type { z } from 'zod'
import { clearLampToId } from '@server/dto/common'
import {
    dashboardCalendarOutputSchema,
    dashboardCoursesOutputSchema,
    dashboardOverviewOutputSchema,
    dashboardReplaysOutputSchema,
    dashboardRivalsOutputSchema,
    dashboardTableProgressOutputSchema,
} from '@server/dto/dashboard'
import type { AuthUser } from '@server/lib/auth-context'
import { RATE_LIMIT, resetRateLimits } from '@server/lib/with-rate-limit'
import { createDashboardRoutes, type DashboardReads, type DashboardRouteDeps } from '@server/route/dashboard'
import { toDashboardRank, toDashboardStandings } from '@server/service/domain/dashboard/dashboard.service'
import { TODAY_START, makeDashboardService, noonOf } from '../service/dashboard/dashboard-fixture'

const ALICE: AuthUser = { id: 'u_alice', loginId: 'alice', name: 'Alice', role: 'user' }
const BOB: AuthUser = { id: 'u_bob', loginId: 'bob', name: 'Bob', role: 'user' }
const USERS = new Map([ALICE, BOB].map((user) => [user.loginId, user]))
const ORIGIN = 'http://localhost:3000'
const BASE = '/api/fe/me/dashboard'
const PRIVATE_CACHE = 'private, no-store'
const RATE_LIMITED_STATUS = 429

const noWait = async () => undefined

const setUp = () => {
    const seen: { route: string; userId: string; argument?: unknown }[] = []
    const service = makeDashboardService({
        db: {
            countChartBests: async () => ({ played: 3, cleared: 2, hardOrBetter: 1, fullComboOrBetter: 0 }),
            summariseScores: async () => ({ total: 9, lastPlayedAt: noonOf(0) }),
            listPlayedAts: async () => ({ playedAts: [noonOf(1), noonOf(0)], previousPlayedAt: null }),
            listRecentCourseScores: async () => [{ courseHash: 'c1', name: 'one', clear: clearLampToId('Hard'), exScore: 3000, playedAt: noonOf(1) }],
            listRivals: async () => [{ userId: 'u_rival', loginId: 'rival', name: 'Rival' }],
            listVersus: async () => [{ rivalId: 'u_rival', common: 2, wins: 1, losses: 1 }],
            listRecentReplays: async () => [
                {
                    id: 'r1',
                    createdAt: noonOf(1),
                    chartSha256: 'a'.repeat(64),
                    chartMd5: 'b'.repeat(32),
                    title: 'Song',
                    artist: 'Artist',
                    level: 12,
                    clear: clearLampToId('Hard'),
                    exScore: 1500,
                },
            ],
        },
        tableSnapshotService: {
            getDetail: async (tableId) =>
                tableId === 'stella-st'
                    ? {
                          table: {
                              id: 'stella-st',
                              name: 'Stella st',
                              symbol: 'st',
                              entry_count: 1,
                              level_count: 1,
                              last_update: '',
                              levels: [{ level: '1', count: 1 }],
                              standard_paths: { page: '', header: '', data: '' },
                              source: null,
                          },
                          levels: [
                              {
                                  level: '1',
                                  entries: [{ md5: 'a'.repeat(32), level: '1', title: 't', artist: '', url: '', url_diff: '', name_diff: '' }],
                              },
                          ],
                      }
                    : null,
            getRecords: async ({ tableId }) => (tableId === 'stella-st' ? { table_id: tableId, records: {} } : null),
        },
    })
    const standings = toDashboardStandings([{ userId: ALICE.id, position: 1 }])
    const reads: DashboardReads = {
        overview: async (viewer) => {
            seen.push({ route: 'overview', userId: viewer.id })
            return { ...(await service.overview({ userId: viewer.id, todayStart: TODAY_START })), rank: toDashboardRank(standings, viewer.id) }
        },
        calendar: async (viewer, query) => {
            seen.push({ route: 'calendar', userId: viewer.id, argument: query })
            return service.calendar({ userId: viewer.id, todayStart: TODAY_START, days: query.days })
        },
        tableProgress: async (viewer, tableId) => {
            seen.push({ route: 'tableProgress', userId: viewer.id, argument: tableId })
            return service.tableProgress({ userId: viewer.id, tableId })
        },
        courses: async (viewer) => {
            seen.push({ route: 'courses', userId: viewer.id })
            return service.courses({ userId: viewer.id })
        },
        rivals: async (viewer, query) => {
            seen.push({ route: 'rivals', userId: viewer.id, argument: query })
            return service.rivals({ userId: viewer.id, limit: query.limit })
        },
        replays: async (viewer, query) => {
            seen.push({ route: 'replays', userId: viewer.id, argument: query })
            return service.replays({ userId: viewer.id, limit: query.limit })
        },
    }
    const deps: DashboardRouteDeps = {
        authService: { resolveSessionUser: async (request) => USERS.get(request.headers.get('x-session-user') ?? '') ?? null },
        reads,
    }
    return { routes: createDashboardRoutes(() => deps, noWait), seen }
}

const call = (path: string, user?: AuthUser) =>
    new Request(`${ORIGIN}${BASE}${path}`, { method: 'GET', headers: user ? { 'x-session-user': user.loginId } : {} })

const tableContext = (tableId: string) => ({ params: Promise.resolve({ tableId }) })

beforeEach(() => resetRateLimits())

describe('GET /api/fe/me/dashboard/*', () => {
    test('세션이 없으면 여섯 경로 모두 401 이고 읽기 함수를 부르지 않는다', async () => {
        const { routes, seen } = setUp()
        const responses = await Promise.all([
            routes.overview(call('/overview'), undefined),
            routes.calendar(call('/calendar'), undefined),
            routes.tableProgress(call('/tables/stella-st/progress'), tableContext('stella-st')),
            routes.courses(call('/courses'), undefined),
            routes.rivals(call('/rivals'), undefined),
            routes.replays(call('/replays'), undefined),
        ])
        for (const response of responses) {
            expect(response.status).toBe(401)
            expect((await response.json()).error.code).toBe('UNAUTHORIZED')
        }
        expect(seen).toEqual([])
    })

    test('여섯 경로의 성공 응답은 봉투에 싸여 캐시하지 않는 헤더로 나가고 DTO 를 통과한다', async () => {
        const { routes } = setUp()
        const checks: [Response, z.ZodType][] = [
            [await routes.overview(call('/overview', ALICE), undefined), dashboardOverviewOutputSchema],
            [await routes.calendar(call('/calendar', ALICE), undefined), dashboardCalendarOutputSchema],
            [await routes.tableProgress(call('/tables/stella-st/progress', ALICE), tableContext('stella-st')), dashboardTableProgressOutputSchema],
            [await routes.courses(call('/courses', ALICE), undefined), dashboardCoursesOutputSchema],
            [await routes.rivals(call('/rivals', ALICE), undefined), dashboardRivalsOutputSchema],
            [await routes.replays(call('/replays', ALICE), undefined), dashboardReplaysOutputSchema],
        ]
        for (const [response, schema] of checks) {
            expect(response.status).toBe(200)
            expect(response.headers.get('cache-control')).toBe(PRIVATE_CACHE)
            const body = await response.json()
            expect(body.success).toBe(true)
            expect(schema.safeParse(body.data).success).toBe(true)
        }
    })

    test('읽기 함수에는 세션 사용자와 검증된 질의만 간다', async () => {
        const { routes, seen } = setUp()
        await routes.overview(call('/overview', BOB), undefined)
        await routes.calendar(call('/calendar?days=30', BOB), undefined)
        await routes.rivals(call('/rivals?limit=3', BOB), undefined)
        await routes.replays(call('/replays', BOB), undefined)
        await routes.tableProgress(call('/tables/stella-st/progress', BOB), tableContext('stella-st'))
        expect(seen).toEqual([
            { route: 'overview', userId: BOB.id },
            { route: 'calendar', userId: BOB.id, argument: { days: 30 } },
            { route: 'rivals', userId: BOB.id, argument: { limit: 3 } },
            { route: 'replays', userId: BOB.id, argument: { limit: 5 } },
            { route: 'tableProgress', userId: BOB.id, argument: 'stella-st' },
        ])
    })

    test('질의를 주지 않으면 달력은 182일, 라이벌은 10건이다', async () => {
        const { routes, seen } = setUp()
        const calendar = await (await routes.calendar(call('/calendar', ALICE), undefined)).json()
        expect(calendar.data.days).toHaveLength(182)
        await routes.rivals(call('/rivals', ALICE), undefined)
        expect(seen.at(-1)).toEqual({ route: 'rivals', userId: ALICE.id, argument: { limit: 10 } })
    })

    test('달력 일 수는 1~371 의 정수만 받고 벗어나면 400 이다', async () => {
        const { routes, seen } = setUp()
        const ok = await Promise.all(['1', '371'].map((days) => routes.calendar(call(`/calendar?days=${days}`, ALICE), undefined)))
        expect(ok.map((response) => response.status)).toEqual([200, 200])
        for (const days of ['0', '372', '-1', 'abc', '1.5']) {
            const response = await routes.calendar(call(`/calendar?days=${days}`, ALICE), undefined)
            expect(response.status).toBe(400)
            expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
        }
        expect(seen).toHaveLength(2)
    })

    test('라이벌·리플레이 건수는 1~20 만 받는다', async () => {
        const { routes } = setUp()
        for (const limit of ['0', '21', 'x']) {
            expect((await routes.rivals(call(`/rivals?limit=${limit}`, ALICE), undefined)).status).toBe(400)
            expect((await routes.replays(call(`/replays?limit=${limit}`, ALICE), undefined)).status).toBe(400)
        }
        expect((await routes.rivals(call('/rivals?limit=20', ALICE), undefined)).status).toBe(200)
        expect((await routes.replays(call('/replays?limit=20', ALICE), undefined)).status).toBe(200)
    })

    test('표 id 형식이 틀리거나 서버에 없는 표면 404 다', async () => {
        const { routes, seen } = setUp()
        for (const tableId of ['UPPER', '../etc', 'a'.repeat(65)]) {
            const response = await routes.tableProgress(call(`/tables/${tableId}/progress`, ALICE), tableContext(tableId))
            expect(response.status).toBe(404)
            expect((await response.json()).error.code).toBe('IR_TABLE_NOT_FOUND')
        }
        expect(seen).toEqual([])
        const unknown = await routes.tableProgress(call('/tables/unknown-table/progress', ALICE), tableContext('unknown-table'))
        expect(unknown.status).toBe(404)
        expect((await unknown.json()).error.code).toBe('IR_TABLE_NOT_FOUND')
    })

    test('한 계정의 한도는 여섯 경로가 나눠 쓰고 넘으면 429 다', async () => {
        const { routes } = setUp()
        const limit = RATE_LIMIT.DASHBOARD.limit
        for (let sent = 0; sent < limit; sent += 1) {
            const response = await routes.courses(call('/courses', ALICE), undefined)
            expect(response.status).toBe(200)
        }
        for (const route of [routes.overview, routes.courses, routes.replays]) {
            const response = await route(call('/overview', ALICE), undefined)
            expect(response.status).toBe(RATE_LIMITED_STATUS)
            expect((await response.json()).error.code).toBe('RATE_LIMITED')
        }
        expect((await routes.overview(call('/overview', BOB), undefined)).status).toBe(200)
    })

    test('분당 한도는 60 이다', () => {
        expect(RATE_LIMIT.DASHBOARD).toEqual({ limit: 60, windowMs: 60_000 })
        expect(RATE_LIMIT.TRAINING_DOWNLOAD).toEqual({ limit: 30, windowMs: 60_000 })
    })
})
