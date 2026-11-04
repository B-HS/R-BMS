import { describe, expect, test } from 'bun:test'
import {
    CACHE_PROFILE,
    CACHE_TAG,
    revalidateOnCourseSubmit,
    revalidateOnDanAward,
    revalidateOnReplayUpload,
    revalidateOnScoreSubmit,
} from '@server/service/read-cache'

type Revalidation = [tag: string, profile: string | { expire?: number }]

const record = () => {
    const calls: Revalidation[] = []
    return { calls, revalidate: (tag: string, profile: string | { expire?: number }) => void calls.push([tag, profile]) }
}

const DASHBOARD_REVALIDATION: Revalidation = [CACHE_TAG.playerDashboard('alice'), CACHE_PROFILE.player]
const SHA256 = 'a'.repeat(64)

describe('대시보드 캐시 태그', () => {
    test('태그 이름은 로그인 id 마다 하나다', () => {
        expect(CACHE_TAG.playerDashboard('alice')).toBe('player-dashboard:alice')
        expect(CACHE_TAG.playerDashboard('alice')).not.toBe(CACHE_TAG.playerDashboard('bob'))
    })

    test('점수를 올리면 그 플레이어의 대시보드를 버리고 종합 순위 캐시도 버린다', () => {
        const { calls, revalidate } = record()
        revalidateOnScoreSubmit({ sha256: SHA256, md5: null, loginId: 'alice' }, revalidate)
        expect(calls).toContainEqual(DASHBOARD_REVALIDATION)
        expect(calls).toContainEqual([CACHE_TAG.leaderboardPlayers(), CACHE_PROFILE.feed])
        expect(calls).not.toContainEqual([CACHE_TAG.playerDashboard('bob'), CACHE_PROFILE.player])
    })

    test('로그인 id 가 없는 점수(손님)는 플레이어 태그를 건드리지 않는다', () => {
        const { calls, revalidate } = record()
        revalidateOnScoreSubmit({ sha256: SHA256, md5: null, loginId: null }, revalidate)
        expect(calls.some(([tag]) => tag.startsWith('player-dashboard:'))).toBe(false)
    })

    test('코스 기록을 올리면 그 플레이어의 대시보드를 버린다', () => {
        const { calls, revalidate } = record()
        revalidateOnCourseSubmit({ courseHash: 'course', loginId: 'alice' }, revalidate)
        expect(calls).toContainEqual(DASHBOARD_REVALIDATION)
        const guest = record()
        revalidateOnCourseSubmit({ courseHash: 'course', loginId: null }, guest.revalidate)
        expect(guest.calls.some(([tag]) => tag.startsWith('player-dashboard:'))).toBe(false)
    })

    test('리플레이를 올리면 그 플레이어의 대시보드를 버린다', () => {
        const { calls, revalidate } = record()
        revalidateOnReplayUpload({ chartSha256: SHA256, loginId: 'alice', scoreId: 's1' }, revalidate)
        expect(calls).toContainEqual(DASHBOARD_REVALIDATION)
        expect(calls).toContainEqual([CACHE_TAG.playerRecent('alice'), CACHE_PROFILE.player])
        const guest = record()
        revalidateOnReplayUpload({ chartSha256: SHA256, loginId: null }, guest.revalidate)
        expect(guest.calls.some(([tag]) => tag.startsWith('player-dashboard:'))).toBe(false)
    })

    test('단위를 받으면 그 플레이어의 대시보드와 종합 순위 캐시를 버린다', () => {
        const { calls, revalidate } = record()
        revalidateOnDanAward({ loginId: 'alice' }, revalidate)
        expect(calls).toEqual([
            [CACHE_TAG.player('alice'), CACHE_PROFILE.player],
            [CACHE_TAG.leaderboardPlayers(), CACHE_PROFILE.feed],
            DASHBOARD_REVALIDATION,
        ])
    })
})
