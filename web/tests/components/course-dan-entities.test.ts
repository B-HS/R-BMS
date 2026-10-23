import { afterEach, describe, expect, test } from 'bun:test'
import { toLeaderboardItem } from '../../src/server/service/domain/fe/fe.service'
import { courseCatalogEntryQueryOptions, courseCatalogQueryOptions } from '../../src/entities/course.query'
import { myDansQueryOptions } from '../../src/entities/dan.query'
import type { PlayerLeaderboardRow } from '../../src/entities/stats.type'
import { QUERY_KEY } from '../../src/shared/constants/query-key'
import { ApiError } from '../../src/shared/lib/fetch'

const originalFetch = globalThis.fetch

const respondWith = (body: string, status = 200) => {
    const calls: string[] = []
    globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
        calls.push(String(input))
        return new Response(body, { status, headers: { 'content-type': 'application/json' } })
    }, originalFetch)
    return calls
}

const run = async <T>(options: { queryFn?: unknown }) => (options.queryFn as () => Promise<T>)()

type Equals<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Flat<T> = { [K in keyof T]: T[K] }

const assertSameShape = <A, B>(...proof: Equals<A, B> extends true ? [] : ['서버 순위 행과 FE 타입의 필드가 어긋났습니다']) => proof.length === 0

afterEach(() => {
    globalThis.fetch = originalFetch
})

describe('코스 카탈로그 쿼리', () => {
    test('코스 묶음 목록은 봉투를 벗겨서 돌려준다', async () => {
        const calls = respondWith('{"success":true,"data":{"groups":[]}}')
        expect(await run<{ groups: unknown[] }>(courseCatalogQueryOptions())).toEqual({ groups: [] })
        expect(calls[0]).toEndWith('/api/fe/courses')
    })

    test('코스 하나의 카탈로그 항목은 해시를 주소에 인코딩해 읽는다', async () => {
        const calls = respondWith('{"success":true,"data":{"course_hash":"ch 1","name":"n"}}')
        expect(await run<{ course_hash: string }>(courseCatalogEntryQueryOptions('ch 1'))).toMatchObject({ course_hash: 'ch 1' })
        expect(calls[0]).toEndWith('/api/fe/courses/ch%201/catalog')
    })

    test('카탈로그에 없는 코스(404)는 오류가 아니라 null 이다', async () => {
        respondWith('{"success":false,"error":{"code":"IR_COURSE_NOT_FOUND","message":"코스를 찾을 수 없습니다."}}', 404)
        expect(await run<unknown>(courseCatalogEntryQueryOptions('unknown'))).toBeNull()
    })

    test('404 가 아닌 실패는 그대로 오류로 던진다', async () => {
        respondWith('{"success":false,"error":{"code":"INTERNAL_ERROR","message":"서버 오류"}}', 500)
        const failure = await run<unknown>(courseCatalogEntryQueryOptions('x')).catch((error: unknown) => error)
        expect(failure).toBeInstanceOf(ApiError)
        expect(failure).toMatchObject({ status: 500, code: 'INTERNAL_ERROR' })
    })

    test('코스 하나마다 쿼리 키가 다르고 모두 코스 전체 키 아래에 있다', () => {
        expect(courseCatalogEntryQueryOptions('a').queryKey).not.toEqual(courseCatalogEntryQueryOptions('b').queryKey)
        for (const key of [QUERY_KEY.COURSE.CATALOG, QUERY_KEY.COURSE.CATALOG_ENTRY('a')]) expect(key[0]).toBe(QUERY_KEY.COURSE.ALL[0])
    })
})

describe('단위 쿼리', () => {
    test('내 단위는 봉투를 벗겨서 읽는다', async () => {
        const calls = respondWith('{"success":true,"data":{"systems":[]}}')
        expect(await run<{ systems: unknown[] }>(myDansQueryOptions())).toEqual({ systems: [] })
        expect(calls[0]).toEndWith('/api/fe/me/dans')
    })

    test('내 단위의 쿼리 키는 단위 전체 키 아래에 있다', () => {
        expect(myDansQueryOptions().queryKey[0]).toBe(QUERY_KEY.DAN.ALL[0])
    })
})

describe('플레이어 순위 행 타입', () => {
    test('서버 순위 행과 필드가 완전히 같다', () => {
        expect(assertSameShape<Flat<PlayerLeaderboardRow>, Flat<ReturnType<typeof toLeaderboardItem>>>()).toBe(true)
    })
})
