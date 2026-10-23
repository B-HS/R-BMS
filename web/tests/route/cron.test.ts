import { describe, expect, test } from 'bun:test'
import { createTableRefreshCronRoute } from '@server/route/cron'
import type { TableRefreshOptions, TableRefreshReport } from '@server/service/domain/table-refresh/table-refresh.service'

const ORIGIN = 'https://bms.hyuns.uk'
const SECRET = 'cron-secret-0123456789'
const SAME_LENGTH_OTHER_SECRET = 'cron-secret-9876543210'

const REPORT: TableRefreshReport = {
    tableId: 'stella-st',
    outcome: 'saved',
    reason: null,
    entries: 2350,
    added: 3,
    removed: 0,
    releveled: 1,
    courses: null,
    awarded: null,
}

const createHarness = (secret: string | null = SECRET) => {
    const calls: TableRefreshOptions[] = []
    const route = createTableRefreshCronRoute(
        () => ({
            tableRefreshService: {
                refreshAll: async (options) => {
                    calls.push(options)
                    return [REPORT]
                },
            },
        }),
        () => secret ?? undefined,
    )
    const call = (headers: Record<string, string> = {}) => route(new Request(`${ORIGIN}/api/cron/tables`, { headers }), undefined)
    return { call, calls }
}

describe('GET /api/cron/tables', () => {
    test('CRON_SECRET 이 없으면 503 이고 표를 받지 않는다', async () => {
        const { call, calls } = createHarness(null)
        const response = await call({ authorization: `Bearer ${SECRET}` })
        expect(response.status).toBe(503)
        expect((await response.json()).error.code).toBe('SERVICE_NOT_CONFIGURED')
        expect(calls).toHaveLength(0)
    })

    test('Authorization 이 없으면 401', async () => {
        const { call, calls } = createHarness()
        const response = await call()
        expect(response.status).toBe(401)
        expect(calls).toHaveLength(0)
    })

    test('Bearer 값이 다르면 401 — 길이가 같은 다른 값과 접두사만 맞는 값도 포함', async () => {
        const { call, calls } = createHarness()
        for (const authorization of [
            `Bearer ${SAME_LENGTH_OTHER_SECRET}`,
            `Bearer ${SECRET.slice(0, -1)}`,
            `Bearer ${SECRET}x`,
            SECRET,
            `bearer ${SECRET}`,
            'Bearer ',
        ]) {
            expect((await call({ authorization })).status).toBe(401)
        }
        expect(calls).toHaveLength(0)
    })

    test('맞으면 200 이고 표마다 결과를 돌려주며 큰 변경 허용 없이 실제로 쓴다', async () => {
        const { call, calls } = createHarness()
        const response = await call({ authorization: `Bearer ${SECRET}` })
        expect(response.status).toBe(200)
        expect(await response.json()).toEqual({ success: true, data: { tables: [REPORT] } })
        expect(calls).toEqual([{ allowLargeChange: false, dryRun: false }])
    })

    test('응답은 캐시하지 않는다', async () => {
        const response = await createHarness().call({ authorization: `Bearer ${SECRET}` })
        expect(response.headers.get('cache-control')).toBe('private, no-store')
    })

    test('비밀 값은 어느 응답 본문에도 없다', async () => {
        const { call } = createHarness()
        const attempts: Record<string, string>[] = [
            { authorization: `Bearer ${SECRET}` },
            { authorization: `Bearer ${SAME_LENGTH_OTHER_SECRET}` },
            {},
        ]
        const bodies = await Promise.all(attempts.map(async (headers) => (await call(headers)).text()))
        for (const body of bodies) {
            expect(body).not.toContain(SECRET)
            expect(body).not.toContain(SAME_LENGTH_OTHER_SECRET)
        }
        const unconfigured = await createHarness(null).call({ authorization: `Bearer ${SECRET}` })
        expect(await unconfigured.text()).not.toContain(SECRET)
    })
})
