import { afterEach, describe, expect, test } from 'bun:test'
import { ApiError, buildApiUrl, fetchEnvelope, fetchEnvelopePage, fetchNoContent, fetchRaw } from '../../src/shared/lib/fetch'
import type { PlayerProfile } from '../../src/entities/player.type'

const originalFetch = globalThis.fetch

const stubFetch = (body: string, status = 200) => {
    globalThis.fetch = Object.assign(async () => new Response(body, { status, headers: { 'content-type': 'application/json' } }), originalFetch)
}

const stubNoContent = () => {
    globalThis.fetch = Object.assign(async () => new Response(null, { status: 204 }), originalFetch)
}

afterEach(() => {
    globalThis.fetch = originalFetch
})

describe('buildApiUrl 쿼리 직렬화', () => {
    test('빈 문자열·undefined·null 파라미터는 빠진다', () => {
        expect(buildApiUrl('/fe/charts/search', { q: '', mode: undefined, level: null, page: 2 })).toEndWith('/api/fe/charts/search?page=2')
    })

    test('파라미터가 없으면 물음표를 붙이지 않는다', () => {
        expect(buildApiUrl('/fe/stats/summary')).toEndWith('/api/fe/stats/summary')
    })
})

describe('fetchEnvelope 봉투 해석', () => {
    test('success 봉투에서 data 만 꺼낸다', async () => {
        stubFetch('{"success":true,"data":{"chart_count":18422}}')
        expect(await fetchEnvelope<{ chart_count: number }>('http://x/api/fe/stats/summary')).toEqual({ chart_count: 18422 })
    })

    test('success:false 면 error.code 와 message 를 담은 ApiError 를 던진다', async () => {
        stubFetch('{"success":false,"error":{"code":"IR_CHART_NOT_FOUND","message":"대상을 찾을 수 없습니다."}}', 404)
        const failure = await fetchEnvelope('http://x/api/fe/charts/none/leaderboard').catch((error: unknown) => error)
        expect(failure).toBeInstanceOf(ApiError)
        expect((failure as ApiError).code).toBe('IR_CHART_NOT_FOUND')
        expect((failure as ApiError).status).toBe(404)
    })

    test('봉투가 아닌 본문은 HTTP_ERROR 로 감싼다', async () => {
        stubFetch('{"chart_count":1}')
        const failure = await fetchEnvelope('http://x/api/fe/stats/summary').catch((error: unknown) => error)
        expect((failure as ApiError).code).toBe('HTTP_ERROR')
    })
})

describe('fetchEnvelopePage 페이지네이션 보존', () => {
    test('data 와 pagination 을 함께 돌려준다', async () => {
        stubFetch('{"success":true,"data":[{"id":"a"}],"pagination":{"page":1,"limit":20,"total":3,"totalPages":1}}')
        expect(await fetchEnvelopePage<{ id: string }[]>('http://x/api/fe/leaderboards/players')).toEqual({
            data: [{ id: 'a' }],
            pagination: { page: 1, limit: 20, total: 3, totalPages: 1 },
        })
    })
})

describe('fetchRaw 네이티브 응답', () => {
    test('봉투 없는 JSON 을 그대로 돌려준다', async () => {
        stubFetch('{"id":"alice","name":"alice","total_plays":1284,"rank_points":8421.5,"extra":{"rank":"A"}}')
        expect(await fetchRaw<PlayerProfile>('http://x/api/players/alice')).toEqual({
            id: 'alice',
            name: 'alice',
            total_plays: 1284,
            rank_points: 8421.5,
            extra: { rank: 'A' },
        })
    })

    test('비 2xx 응답은 상태코드를 담은 ApiError 로 던진다', async () => {
        stubFetch('{"success":false,"error":{"code":"IR_PLAYER_NOT_FOUND","message":"없음"}}', 404)
        const failure = await fetchRaw('http://x/api/players/none').catch((error: unknown) => error)
        expect((failure as ApiError).code).toBe('IR_PLAYER_NOT_FOUND')
    })
})

describe('fetchNoContent 204 응답', () => {
    test('본문 없는 204 를 오류 없이 통과시킨다', async () => {
        stubNoContent()
        expect(await fetchNoContent('http://x/api/fe/tokens/tk_1', { method: 'DELETE' })).toBeUndefined()
    })

    test('204 가 아닌 실패는 봉투의 error.code 로 던진다', async () => {
        stubFetch('{"success":false,"error":{"code":"NOT_FOUND","message":"없음"}}', 404)
        const failure = await fetchNoContent('http://x/api/fe/tokens/tk_1', { method: 'DELETE' }).catch((error: unknown) => error)
        expect((failure as ApiError).code).toBe('NOT_FOUND')
    })
})
