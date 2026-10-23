import { describe, expect, test } from 'bun:test'
import { CACHE_PROFILE, CACHE_TAG, CACHE_TAG_EXT, revalidateOnTableRefresh } from '@server/service/read-cache'
import type { TableRefreshReport } from '@server/service/domain/table-refresh/table-refresh.service'

const NEW_DAN_HASHES = ['course-st0', 'course-st1']
const NO_COURSE_WORK = { id: 'stella-st', inserted: 0, changed: 0, unchanged: 26, deactivated: 0, danCourses: 13, newDanHashes: [] }

const report = (overrides: Partial<TableRefreshReport> = {}): TableRefreshReport => ({
    tableId: 'stella-st',
    outcome: 'unchanged',
    reason: null,
    entries: 2350,
    added: 0,
    removed: 0,
    releveled: 0,
    courses: NO_COURSE_WORK,
    awarded: null,
    ...overrides,
})

type Revalidation = [tag: string, profile: string | { expire?: number }]

const revalidatedBy = (reports: TableRefreshReport[]) => {
    const calls: Revalidation[] = []
    revalidateOnTableRefresh(reports, (tag, profile) => void calls.push([tag, profile]))
    return calls
}

describe('revalidateOnTableRefresh', () => {
    test('아무것도 바뀌지 않은 갱신은 캐시를 건드리지 않는다', () => {
        expect(revalidatedBy([report()])).toEqual([])
    })

    test('받기·검증에 실패했거나 거절된 갱신도 캐시를 건드리지 않는다', () => {
        const failed = report({ outcome: 'failed', reason: 'fetch_failed', courses: null })
        const rejected = report({ outcome: 'rejected', reason: 'shrank', courses: null })
        expect(revalidatedBy([failed, rejected])).toEqual([])
    })

    test('표를 새로 저장하면 코스 카탈로그 캐시를 버린다', () => {
        expect(revalidatedBy([report({ outcome: 'saved' })])).toEqual([[CACHE_TAG_EXT.courseCatalog(), CACHE_PROFILE.ranking]])
    })

    test('표는 같아도 코스가 등록되거나 바뀌거나 빠졌으면 코스 카탈로그 캐시를 버린다', () => {
        for (const courses of [{ inserted: 1 }, { changed: 1 }, { deactivated: 1 }]) {
            const calls = revalidatedBy([report({ courses: { ...NO_COURSE_WORK, ...courses } })])
            expect(calls).toEqual([[CACHE_TAG_EXT.courseCatalog(), CACHE_PROFILE.ranking]])
        }
    })

    test('새로 단위를 주게 된 코스는 코스 페이지 캐시도 버린다', () => {
        const calls = revalidatedBy([report({ outcome: 'saved', courses: { ...NO_COURSE_WORK, inserted: 2, newDanHashes: NEW_DAN_HASHES } })])
        expect(calls).toEqual([
            [CACHE_TAG_EXT.courseCatalog(), CACHE_PROFILE.ranking],
            ...NEW_DAN_HASHES.map((courseHash): Revalidation => [CACHE_TAG_EXT.course(courseHash), CACHE_PROFILE.chart]),
        ])
    })

    test('단위를 받은 사람이 있으면 모든 프로필과 플레이어 순위 캐시를 버린다', () => {
        const calls = revalidatedBy([report({ outcome: 'saved', awarded: 3 })])
        expect(calls).toEqual([
            [CACHE_TAG_EXT.courseCatalog(), CACHE_PROFILE.ranking],
            [CACHE_TAG.playerProfiles(), CACHE_PROFILE.player],
            [CACHE_TAG.leaderboardPlayers(), CACHE_PROFILE.feed],
        ])
    })

    test('단위를 받은 사람이 없으면(0 이거나 판정을 안 돌림) 프로필 캐시는 그대로다', () => {
        const zero = revalidatedBy([report({ outcome: 'saved', awarded: 0 })])
        const skipped = revalidatedBy([report({ outcome: 'saved', awarded: null })])
        for (const calls of [zero, skipped]) expect(calls.map(([tag]) => tag)).toEqual([CACHE_TAG_EXT.courseCatalog()])
    })

    test('표가 둘이면 한 표의 변화만으로도 해당 캐시를 한 번씩 버린다', () => {
        const unchanged = report({ tableId: 'stella-sl' })
        const awarded = report({ outcome: 'saved', awarded: 1 })
        const tags = revalidatedBy([unchanged, awarded]).map(([tag]) => tag)
        expect(tags).toEqual([CACHE_TAG_EXT.courseCatalog(), CACHE_TAG.playerProfiles(), CACHE_TAG.leaderboardPlayers()])
    })
})
