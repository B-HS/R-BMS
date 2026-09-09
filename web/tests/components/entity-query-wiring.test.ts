import { afterEach, describe, expect, test } from 'bun:test'
import { settingKeyListQueryOptions } from '../../src/entities/setting.query'
import type { SettingBlobKey } from '../../src/entities/setting.type'
import { clientBuildListQueryOptions } from '../../src/entities/build.query'
import { tableListQueryOptions } from '../../src/entities/table.query'
import { courseRankingQueryOptions } from '../../src/entities/course.query'

const originalFetch = globalThis.fetch

const capture = (body: string) => {
    const calls: string[] = []
    globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
        calls.push(String(input))
        return new Response(body, { status: 200, headers: { 'content-type': 'application/json' } })
    }, originalFetch)
    return calls
}

const run = async <T>(options: { queryFn?: unknown }) => (options.queryFn as () => Promise<T>)()

afterEach(() => {
    globalThis.fetch = originalFetch
})

describe('entities 쿼리가 실제 라우트 응답 형태를 따른다', () => {
    test('설정 키 목록은 봉투 안 keys 배열을 벗겨서 돌려준다', async () => {
        const calls = capture('{"success":true,"data":{"keys":[{"key":"config","format":"ron","updated_at":10,"size":6}]}}')
        expect(await run<SettingBlobKey[]>(settingKeyListQueryOptions())).toEqual([{ key: 'config', format: 'ron', updated_at: 10, size: 6 }])
        expect(calls[0]).toEndWith('/api/fe/me/settings')
    })

    test('빌드 allowlist 는 admin 라우트의 봉투를 벗겨서 돌려준다', async () => {
        const calls = capture(
            '{"success":true,"data":[{"sha256":"ab","version":"0.1.0","platform":"macos","channel":"stable","released_at":1,"trusted":true,"note":""}]}',
        )
        const builds = await run<{ version: string }[]>(clientBuildListQueryOptions())
        expect(builds[0].version).toBe('0.1.0')
        expect(calls[0]).toEndWith('/api/admin/builds')
    })

    test('표 목록은 봉투 없는 배열을 그대로 받는다', async () => {
        const calls = capture('[{"id":"satellite","name":"Satellite","url":null,"folders":[],"courses":[],"extra":{}}]')
        const tables = await run<{ id: string }[]>(tableListQueryOptions())
        expect(tables[0].id).toBe('satellite')
        expect(calls[0]).toEndWith('/api/tables')
    })

    test('코스 랭킹은 limit 을 쿼리스트링으로 보낸다', async () => {
        const calls = capture('[]')
        await run<unknown[]>(courseRankingQueryOptions('ch 1', { limit: 25 }))
        expect(calls[0]).toEndWith('/api/courses/ch%201/ranking?limit=25')
    })
})
