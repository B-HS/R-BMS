import { describe, expect, test } from 'bun:test'
import { POST as upsertChart } from '@app/api/charts/route'
import { POST as upsertCourseMeta } from '@app/api/courses/meta/route'
import { POST as register } from '@app/api/auth/register/route'
import { POST as authHandlerPost } from '@app/api/auth/[...all]/route'
import { MAX_PASSWORD_LENGTH } from '@server/dto/auth'
import { CHART_UPSERT_MAX_BYTES } from '@server/dto/chart'

const emptyContext = { params: Promise.resolve({}) }
const json = { 'content-type': 'application/json' }

describe('POST /api/charts', () => {
    test('Bearer 없이는 401 이다', async () => {
        const response = await upsertChart(
            new Request('http://localhost:3000/api/charts', { method: 'POST', body: '{}', headers: json }),
            emptyContext,
        )
        expect(response.status).toBe(401)
    })
})

describe('POST /api/courses/meta', () => {
    test('세션이 없으면 401 이고 API 토큰으로는 열리지 않는다', async () => {
        const anonymous = await upsertCourseMeta(
            new Request('http://localhost:3000/api/courses/meta', { method: 'POST', body: '{}', headers: json }),
            emptyContext,
        )
        expect(anonymous.status).toBe(401)
        const bearer = await upsertCourseMeta(
            new Request('http://localhost:3000/api/courses/meta', {
                method: 'POST',
                body: '{}',
                headers: { ...json, authorization: 'Bearer rbms_anything' },
            }),
            emptyContext,
        )
        expect(bearer.status).toBe(401)
    })
})

describe('POST /api/auth/register 의 서버 검증', () => {
    const post = (body: unknown) =>
        register(new Request('http://localhost:3000/api/auth/register', { method: 'POST', body: JSON.stringify(body), headers: json }), emptyContext)

    test('예약어와 잘못된 형식의 아이디는 가입 시도 전에 400 이다', async () => {
        for (const id of ['guest', 'a b', 'a/b']) {
            const response = await post({ id, password: 'password-1234', email: 'a@example.com' })
            expect(response.status).toBe(400)
            expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
        }
    })
})

describe('POST /api/auth/register 의 비밀번호 길이', () => {
    test('better-auth 가 받는 최대 길이를 넘으면 가입 시도 전에 400 이다', async () => {
        const response = await register(
            new Request('http://localhost:3000/api/auth/register', {
                method: 'POST',
                body: JSON.stringify({ id: 'long_password', password: 'p'.repeat(MAX_PASSWORD_LENGTH + 1), email: 'a@example.com' }),
                headers: json,
            }),
            emptyContext,
        )
        expect(response.status).toBe(400)
        expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
    })
})

describe('/api/auth/[...all]', () => {
    test('가입과 로그인 같은 better-auth 기본 경로는 열려 있지 않다', async () => {
        for (const path of ['sign-up/email', 'sign-in/email', 'change-password']) {
            const response = await authHandlerPost(
                new Request(`http://localhost:3000/api/auth/${path}`, { method: 'POST', body: '{}', headers: json }),
                emptyContext,
            )
            expect(response.status).toBe(404)
        }
    })
})

describe('상수', () => {
    test('POST /api/charts 본문 상한은 64KB 이다', () => {
        expect(CHART_UPSERT_MAX_BYTES).toBe(64 * 1024)
    })
})
