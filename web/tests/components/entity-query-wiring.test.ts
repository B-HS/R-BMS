import { afterEach, describe, expect, test } from 'bun:test'
import { settingKeyListQueryOptions } from '../../src/entities/setting.query'
import type { SettingBlobKey } from '../../src/entities/setting.type'
import { clientBuildListQueryOptions } from '../../src/entities/build.query'
import { tableListQueryOptions } from '../../src/entities/table.query'
import { courseRankingQueryOptions } from '../../src/entities/course.query'
import { chartLeaderboardQueryOptions } from '../../src/entities/chart.query'
import { playerPlayDaysQueryOptions, playerRecentQueryOptions } from '../../src/entities/player.query'
import {
    dashboardCalendarQueryOptions,
    dashboardCoursesQueryOptions,
    dashboardOverviewQueryOptions,
    dashboardReplaysQueryOptions,
    dashboardRivalsQueryOptions,
    dashboardTableProgressQueryOptions,
} from '../../src/entities/dashboard.query'
import { QUERY_KEY } from '../../src/shared/constants/query-key'

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

    test('코스 랭킹은 limit 과 SCRATCH AUTO 포함 여부를 쿼리스트링으로 보낸다', async () => {
        const calls = capture('[]')
        await run<unknown[]>(courseRankingQueryOptions('ch 1', { limit: 25, include_scratch_auto: true }))
        expect(calls[0]).toEndWith('/api/courses/ch%201/ranking?limit=25&include_scratch_auto=true')
    })

    test('플레이 기록은 page 와 limit 을 쿼리스트링으로 보내고 pagination 을 함께 돌려준다', async () => {
        const calls = capture('{"success":true,"data":[],"pagination":{"page":2,"limit":20,"total":21,"totalPages":2}}')
        const result = await run<{ data: unknown[]; pagination: { total: number; totalPages: number } }>(
            playerRecentQueryOptions('al ice', { page: 2, limit: 20 }),
        )
        expect(calls[0]).toEndWith('/api/fe/players/al%20ice/recent?page=2&limit=20')
        expect([result.data, result.pagination.total, result.pagination.totalPages]).toEqual([[], 21, 2])
    })

    test('플레이 기록은 페이지가 다르면 쿼리 키가 다르다', () => {
        expect(playerRecentQueryOptions('alice', { page: 1, limit: 20 }).queryKey).not.toEqual(
            playerRecentQueryOptions('alice', { page: 2, limit: 20 }).queryKey,
        )
    })

    test('일별 플레이 시간은 플레이어 아이디를 인코딩한 경로로 요청하고 봉투를 벗겨서 돌려준다', async () => {
        const calls = capture('{"success":true,"data":{"days":[{"date":"2026-11-02","plays":3,"play_ms":420000}],"total_plays":3,"total_ms":420000}}')
        const result = await run<{ days: { date: string; play_ms: number }[]; total_ms: number }>(playerPlayDaysQueryOptions('al ice'))
        expect(calls[0]).toEndWith('/api/fe/players/al%20ice/play-days')
        expect([result.days[0].date, result.days[0].play_ms, result.total_ms]).toEqual(['2026-11-02', 420_000, 420_000])
    })

    test('일별 플레이 시간 쿼리 키는 플레이어마다 다르고 플레이어 전체 키 아래에 있다', () => {
        expect(playerPlayDaysQueryOptions('alice').queryKey).not.toEqual(playerPlayDaysQueryOptions('bob').queryKey)
        expect(playerPlayDaysQueryOptions('alice').queryKey[0]).toBe(QUERY_KEY.PLAYER.ALL[0])
    })

    test('대시보드 개요와 코스와 리플레이와 라이벌은 /api/fe/me/dashboard 아래 경로로 요청하고 봉투를 벗긴다', async () => {
        const calls = capture('{"success":true,"data":{"groups":[],"recent":[]}}')
        expect(await run<{ groups: unknown[]; recent: unknown[] }>(dashboardCoursesQueryOptions())).toEqual({ groups: [], recent: [] })
        await run(dashboardOverviewQueryOptions())
        await run(dashboardRivalsQueryOptions(10))
        await run(dashboardReplaysQueryOptions(5))
        expect(calls.map((call) => call.replace('/api/fe/me/dashboard', ''))).toEqual([
            '/courses',
            '/overview',
            '/rivals?limit=10',
            '/replays?limit=5',
        ])
    })

    test('대시보드 달력은 창 길이를, 표 진행은 인코딩한 표 아이디를 경로에 싣는다', async () => {
        const calls = capture('{"success":true,"data":{}}')
        await run(dashboardCalendarQueryOptions(182))
        await run(dashboardTableProgressQueryOptions('stella st'))
        expect(calls).toEqual(['/api/fe/me/dashboard/calendar?days=182', '/api/fe/me/dashboard/tables/stella%20st/progress'])
    })

    test('대시보드 쿼리 키는 모두 대시보드 전체 키 아래에 있고 창 길이·표·개수가 다르면 키도 다르다', () => {
        const keys = [
            dashboardOverviewQueryOptions().queryKey,
            dashboardCalendarQueryOptions(182).queryKey,
            dashboardTableProgressQueryOptions('insane1').queryKey,
            dashboardCoursesQueryOptions().queryKey,
            dashboardRivalsQueryOptions(10).queryKey,
            dashboardReplaysQueryOptions(5).queryKey,
        ]
        expect(keys.map((key) => key[0])).toEqual(Array(keys.length).fill(QUERY_KEY.DASHBOARD.ALL[0]))
        expect(dashboardCalendarQueryOptions(182).queryKey).not.toEqual(dashboardCalendarQueryOptions(30).queryKey)
        expect(dashboardTableProgressQueryOptions('insane1').queryKey).not.toEqual(dashboardTableProgressQueryOptions('stella-st').queryKey)
        expect(dashboardRivalsQueryOptions(10).queryKey).not.toEqual(dashboardRivalsQueryOptions(20).queryKey)
    })

    test('SCRATCH AUTO 포함 여부가 다르면 차트 순위 쿼리 키가 다르다', () => {
        const excluded = chartLeaderboardQueryOptions('h', { include_scratch_auto: false }).queryKey
        const included = chartLeaderboardQueryOptions('h', { include_scratch_auto: true }).queryKey
        expect(excluded).not.toEqual(included)
    })
})
