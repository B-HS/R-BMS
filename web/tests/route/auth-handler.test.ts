import { beforeEach, describe, expect, test } from 'bun:test'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAuthHandlerRoute, isAllowedAuthHandlerRoute } from '@server/route/auth'

const ORIGIN = 'http://localhost:3000'

const createHandler = () => {
    const calls: string[] = []
    const route = createAuthHandlerRoute(() => async (request) => {
        calls.push(`${request.method} ${new URL(request.url).pathname}`)
        return new Response('forwarded')
    })
    return { calls, route }
}

beforeEach(() => resetRateLimits())

describe('better-auth 기본 엔드포인트 노출', () => {
    test('get-session 과 sign-out 만 허용한다', () => {
        expect(isAllowedAuthHandlerRoute('GET', '/api/auth/get-session')).toBe(true)
        expect(isAllowedAuthHandlerRoute('POST', '/api/auth/sign-out')).toBe(true)
        expect(isAllowedAuthHandlerRoute('POST', '/api/auth/sign-out/')).toBe(true)
        expect(isAllowedAuthHandlerRoute('POST', '/api/auth/get-session')).toBe(false)
        expect(isAllowedAuthHandlerRoute('GET', '/api/auth/sign-out')).toBe(false)
    })

    test('가입, 로그인, 비밀번호, 세션 목록 같은 나머지는 404 이고 better-auth 를 부르지 않는다', async () => {
        const { calls, route } = createHandler()
        const blocked: [string, string][] = [
            ['POST', '/api/auth/sign-up/email'],
            ['POST', '/api/auth/sign-in/email'],
            ['POST', '/api/auth/change-password'],
            ['POST', '/api/auth/delete-user'],
            ['GET', '/api/auth/list-sessions'],
            ['POST', '/api/auth/revoke-sessions'],
            ['POST', '/api/auth/update-user'],
        ]
        for (const [method, path] of blocked) {
            const response = await route(new Request(`${ORIGIN}${path}`, { method }), undefined)
            expect(response.status).toBe(404)
            expect((await response.json()).error.code).toBe('NOT_FOUND')
        }
        expect(calls).toEqual([])
    })

    test('허용한 두 경로는 better-auth 로 넘긴다', async () => {
        const { calls, route } = createHandler()
        expect((await route(new Request(`${ORIGIN}/api/auth/get-session`), undefined)).status).toBe(200)
        expect((await route(new Request(`${ORIGIN}/api/auth/sign-out`, { method: 'POST' }), undefined)).status).toBe(200)
        expect(calls).toEqual(['GET /api/auth/get-session', 'POST /api/auth/sign-out'])
    })
})
