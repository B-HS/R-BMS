import { describe, expect, test } from 'bun:test'
import {
    chartLeaderboardQuerySchema,
    chartSearchQuerySchema,
    feedQuerySchema,
    playerLeaderboardQuerySchema,
    playerRecentQuerySchema,
} from '@server/dto/fe'
import { rankingQuerySchema } from '@server/dto/score'
import { paginatedResponse, toPagination } from '@server/lib/api-response'
import { parseQueryParams } from '@server/service/public-reads'

describe('parseQueryParams', () => {
    test('페이지가 쥔 불리언과 숫자를 쿼리스트링이 실었을 문자열로 보고 HTTP 라우트와 같은 스키마로 검증한다', () => {
        expect(parseQueryParams(chartLeaderboardQuerySchema, { include_scratch_auto: true })).toEqual({ include_scratch_auto: true })
        expect(parseQueryParams(chartLeaderboardQuerySchema, { include_scratch_auto: false })).toEqual({ include_scratch_auto: false })
        expect(parseQueryParams(rankingQuerySchema, { limit: 50, include_scratch_auto: true })).toMatchObject({
            limit: 50,
            page: 1,
            include_scratch_auto: true,
        })
        expect(parseQueryParams(feedQuerySchema, { limit: 20 })).toEqual({ limit: 20 })
    })

    test('비었거나 undefined 인 값은 URL 에서처럼 빠져 기본값이 채워진다', () => {
        expect(parseQueryParams(chartSearchQuerySchema, { q: '', mode: undefined, level: null, sort: 'title', page: 2, limit: 20 })).toEqual({
            sort: 'title',
            page: 2,
            limit: 20,
        })
        expect(parseQueryParams(playerLeaderboardQuerySchema, {})).toMatchObject({ sort: 'cleared', page: 1, include_scratch_auto: false })
    })

    test('스키마에 어긋나는 값은 VALIDATION_ERROR 로 던진다', () => {
        expect(() => parseQueryParams(chartSearchQuerySchema, { sort: 'nonsense' })).toThrow()
        try {
            parseQueryParams(feedQuerySchema, { limit: 100_000 })
        } catch (error) {
            expect(error).toMatchObject({ code: 'VALIDATION_ERROR', statusCode: 400 })
        }
    })
})

describe('플레이 기록 질의', () => {
    test('page 와 limit 은 기본이 1페이지 20판이고 숫자 문자열을 받는다', () => {
        expect(parseQueryParams(playerRecentQuerySchema, {})).toEqual({ page: 1, limit: 20 })
        expect(parseQueryParams(playerRecentQuerySchema, { page: 3, limit: 50 })).toEqual({ page: 3, limit: 50 })
    })

    test('0 이하의 page 와 상한을 넘는 limit 은 VALIDATION_ERROR 다', () => {
        for (const params of [{ page: 0 }, { page: -1 }, { page: 1.5 }, { limit: 0 }, { limit: 101 }]) {
            expect(() => parseQueryParams(playerRecentQuerySchema, params)).toThrow()
        }
    })
})

describe('toPagination', () => {
    test('HTTP 응답 봉투의 pagination 과 같은 모양이다', () => {
        const page = { page: 2, limit: 20, total: 41 }
        expect(toPagination(page)).toEqual({ page: 2, limit: 20, total: 41, totalPages: 3 })
        expect(paginatedResponse([], page).pagination).toEqual(toPagination(page))
        expect(toPagination({ page: 1, limit: 20, total: 0 }).totalPages).toBe(1)
    })
})
