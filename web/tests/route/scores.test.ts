import { describe, expect, test } from 'bun:test'
import { POST as submitScore } from '@app/api/scores/route'
import { GET as chartRanking } from '@app/api/charts/[hash]/ranking/route'

const emptyContext = { params: Promise.resolve({}) }

describe('POST /api/scores (DB 불필요 경로)', () => {
    test('Bearer 없이 guest 비허용이면 401 UNAUTHORIZED 이다', async () => {
        const response = await submitScore(
            new Request('http://localhost:3000/api/scores', { method: 'POST', body: '{}', headers: { 'content-type': 'application/json' } }),
            emptyContext,
        )
        expect(response.status).toBe(401)
        const body = await response.json()
        expect(body.success).toBe(false)
        expect(body.error.code).toBe('UNAUTHORIZED')
    })

    test('본문이 256KB 를 넘으면 413 IR_PAYLOAD_TOO_LARGE 이다', async () => {
        const response = await submitScore(
            new Request('http://localhost:3000/api/scores', {
                method: 'POST',
                body: '{}',
                headers: { 'content-type': 'application/json', 'content-length': String(256 * 1024 + 1) },
            }),
            emptyContext,
        )
        expect(response.status).toBe(413)
        expect((await response.json()).error.code).toBe('IR_PAYLOAD_TOO_LARGE')
    })
})

describe('GET /api/charts/[hash]/ranking (DB 불필요 경로)', () => {
    test('md5 도 sha256 도 아닌 길이는 404 IR_CHART_NOT_FOUND 이다', async () => {
        const response = await chartRanking(new Request('http://localhost:3000/api/charts/abc/ranking?limit=50'), {
            params: Promise.resolve({ hash: 'abc' }),
        })
        expect(response.status).toBe(404)
        expect((await response.json()).error.code).toBe('IR_CHART_NOT_FOUND')
    })

    test('limit 이 범위를 벗어나면 400 VALIDATION_ERROR 이다', async () => {
        const response = await chartRanking(new Request('http://localhost:3000/api/charts/' + 'a'.repeat(32) + '/ranking?limit=0'), {
            params: Promise.resolve({ hash: 'a'.repeat(32) }),
        })
        expect(response.status).toBe(400)
        expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('16진수가 아닌 32자 해시는 404 IR_CHART_NOT_FOUND 이다', async () => {
        const hash = 'A'.repeat(32)
        const response = await chartRanking(new Request(`http://localhost:3000/api/charts/${hash}/ranking?limit=50`), {
            params: Promise.resolve({ hash }),
        })
        expect(response.status).toBe(404)
        expect((await response.json()).error.code).toBe('IR_CHART_NOT_FOUND')
    })
})
