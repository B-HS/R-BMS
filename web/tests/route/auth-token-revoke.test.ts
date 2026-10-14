import { beforeEach, describe, expect, test } from 'bun:test'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createTokenRevokeCurrentRoute } from '@server/route/auth'

const ORIGIN = 'http://localhost:3000'
const LIVE_TOKEN = 'rbms_live'

const createRoute = () => {
    const revoked: string[] = []
    const route = createTokenRevokeCurrentRoute(() => ({
        revokeApiToken: async (plain) => {
            revoked.push(plain)
            return plain === LIVE_TOKEN
        },
    }))
    const call = (headers: Record<string, string> = {}) =>
        route(new Request(`${ORIGIN}/api/auth/token/current`, { method: 'DELETE', headers }), undefined)
    return { call, revoked }
}

beforeEach(() => resetRateLimits())

describe('DELETE /api/auth/token/current', () => {
    test('Bearer 로 보낸 살아 있는 토큰을 폐기하고 204 로 답한다', async () => {
        const { call, revoked } = createRoute()
        const response = await call({ authorization: `Bearer ${LIVE_TOKEN}` })
        expect(response.status).toBe(204)
        expect(revoked).toEqual([LIVE_TOKEN])
    })

    test('토큰이 없으면 서비스를 부르지 않고 401', async () => {
        const { call, revoked } = createRoute()
        expect((await call()).status).toBe(401)
        expect(revoked).toEqual([])
    })

    test('모르는 토큰이나 이미 폐기된 토큰은 401', async () => {
        const { call } = createRoute()
        const response = await call({ authorization: 'Bearer rbms_gone' })
        expect(response.status).toBe(401)
        expect(await response.json()).toMatchObject({ success: false, error: { code: 'UNAUTHORIZED' } })
    })

    test('같은 주소의 분당 한도를 넘으면 429', async () => {
        const { call } = createRoute()
        const statuses = await Promise.all(
            Array.from({ length: 6 }, () => call({ authorization: 'Bearer rbms_gone', 'x-forwarded-for': '203.0.113.9' })),
        )
        expect(statuses.map((response) => response.status)).toEqual([401, 401, 401, 401, 401, 429])
    })
})
