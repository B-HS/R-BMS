import { describe, expect, test } from 'bun:test'
import { POST as uploadReplay } from '@app/api/charts/[hash]/replays/route'
import { GET as courseRanking } from '@app/api/courses/[courseHash]/ranking/route'
import { GET as chartBest } from '@app/api/charts/[hash]/best/route'
import { GET as chartRanking } from '@app/api/charts/[hash]/ranking/route'
import { POST as submitCourse } from '@app/api/courses/route'
import { GET as feChartSearch } from '@app/api/fe/charts/search/route'
import { SETTING_MAX_BYTES, settingPutSchema } from '@server/dto/setting'
import { RATE_LIMIT, consumeRateLimit, resetRateLimits } from '@server/lib/with-rate-limit'
import { resolveKnownChart } from '@server/route/chart'

const MD5 = 'a'.repeat(32)
const REPLAY_MAX_BYTES = 4 * 1024 * 1024
const emptyContext = { params: Promise.resolve({}) }
const hashContext = { params: Promise.resolve({ hash: MD5 }) }

const hasNextRuntime = Boolean(process.env.RBMS_ROUTE_E2E)
if (!hasNextRuntime) {
    console.log("[skip] 'use cache' 라우트 통합 테스트 건너뜀 — Next 런타임(cacheLife) 필요, RBMS_ROUTE_E2E=1 로 실행")
}

describe('POST /api/charts/[hash]/replays (DB 불필요 경로)', () => {
    test('본문이 REPLAY_MAX_BYTES 를 넘으면 413 IR_PAYLOAD_TOO_LARGE 이다', async () => {
        const response = await uploadReplay(
            new Request(`http://localhost:3000/api/charts/${MD5}/replays`, {
                method: 'POST',
                body: '{}',
                headers: { 'content-type': 'application/json', 'content-length': String(REPLAY_MAX_BYTES + 1) },
            }),
            hashContext,
        )
        expect(response.status).toBe(413)
        expect((await response.json()).error.code).toBe('IR_PAYLOAD_TOO_LARGE')
    })

    test('Bearer 없이 업로드하면 401 UNAUTHORIZED 이다', async () => {
        const response = await uploadReplay(
            new Request(`http://localhost:3000/api/charts/${MD5}/replays`, {
                method: 'POST',
                body: JSON.stringify({ format: 'f', events: [] }),
                headers: { 'content-type': 'application/json' },
            }),
            hashContext,
        )
        expect(response.status).toBe(401)
        expect((await response.json()).error.code).toBe('UNAUTHORIZED')
    })
})

describe('POST /api/courses (DB 불필요 경로)', () => {
    test('Bearer 없는 코스 결과 제출은 401 UNAUTHORIZED 이다', async () => {
        const response = await submitCourse(
            new Request('http://localhost:3000/api/courses', { method: 'POST', body: '{}', headers: { 'content-type': 'application/json' } }),
            emptyContext,
        )
        expect(response.status).toBe(401)
        expect((await response.json()).error.code).toBe('UNAUTHORIZED')
    })
})

describe('쿼리 검증 (DB 불필요 경로)', () => {
    test('코스 랭킹의 limit 이 범위를 벗어나면 400 VALIDATION_ERROR 이다', async () => {
        const response = await courseRanking(new Request('http://localhost:3000/api/courses/ch1/ranking?limit=0'), {
            params: Promise.resolve({ courseHash: 'ch1' }),
        })
        expect(response.status).toBe(400)
        expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
    })

    test('FE 차트 검색의 sort 가 목록에 없으면 400 VALIDATION_ERROR 이다', async () => {
        const response = await feChartSearch(new Request('http://localhost:3000/api/fe/charts/search?sort=bogus'), emptyContext)
        expect(response.status).toBe(400)
        expect((await response.json()).error.code).toBe('VALIDATION_ERROR')
    })
})

describe.skipIf(!hasNextRuntime)('미등록 차트의 IR 조회 계약 (Next 런타임 필요)', () => {
    test('랭킹은 404 가 아니라 빈 배열이다', async () => {
        const response = await chartRanking(new Request(`http://localhost:3000/api/charts/${MD5}/ranking?limit=50`), hashContext)
        expect(response.status).toBe(200)
        expect(await response.json()).toEqual([])
    })

    test('베스트는 404 가 아니라 null 이다', async () => {
        const response = await chartBest(new Request(`http://localhost:3000/api/charts/${MD5}/best?player=alice`), hashContext)
        expect(response.status).toBe(200)
        expect(await response.json()).toBeNull()
    })
})

describe('resolveKnownChart (캐시 리더 주입)', () => {
    test('해시 형식이 아니면 IR_CHART_NOT_FOUND 를 던진다', async () => {
        await expect(resolveKnownChart('not-a-hash', async () => null)).rejects.toMatchObject({ code: 'IR_CHART_NOT_FOUND', statusCode: 404 })
    })

    test('형식은 맞지만 미등록 차트면 null 을 돌려준다 (404 아님)', async () => {
        expect(await resolveKnownChart(MD5, async () => null)).toBeNull()
    })

    test('등록된 차트면 sha256 을 돌려준다', async () => {
        const sha256 = 'b'.repeat(64)
        expect(await resolveKnownChart(MD5, async () => ({ sha256, md5: MD5 }))).toEqual({ sha256, md5: MD5 })
    })
})

describe('GET /api/fe/* 공개 읽기 레이트리밋', () => {
    test('IP 당 한도를 넘기면 429 RATE_LIMITED 이다', async () => {
        resetRateLimits()
        const ip = '203.0.113.7'
        for (let call = 0; call < RATE_LIMIT.PUBLIC_READ.limit; call += 1) {
            consumeRateLimit(`fe-read:${ip}`, RATE_LIMIT.PUBLIC_READ.limit, RATE_LIMIT.PUBLIC_READ.windowMs)
        }
        const response = await feChartSearch(
            new Request('http://localhost:3000/api/fe/charts/search?limit=5', { headers: { 'x-forwarded-for': ip } }),
            emptyContext,
        )
        expect(response.status).toBe(429)
        expect((await response.json()).error.code).toBe('RATE_LIMITED')
        resetRateLimits()
    })
})

describe('PUT /api/players/[id]/settings/[name] 본문 상한', () => {
    test('content 가 SETTING_MAX_BYTES 를 넘으면 스키마가 거절한다', () => {
        expect(settingPutSchema.safeParse({ content: 'x'.repeat(SETTING_MAX_BYTES + 1) }).success).toBe(false)
        expect(settingPutSchema.safeParse({ content: 'x'.repeat(SETTING_MAX_BYTES) }).success).toBe(true)
    })
})
