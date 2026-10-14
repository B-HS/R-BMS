import { describe, expect, test } from 'bun:test'
import { sumKnownCourseNotes } from '@server/route/course'

type FindChart = Parameters<typeof sumKnownCourseNotes>[1]
type ChartRow = NonNullable<Awaited<ReturnType<FindChart>>>

const SHA_A = 'a'.repeat(64)
const SHA_B = 'b'.repeat(64)
const MD5_C = 'c'.repeat(32)

const createFinder = (notesByHash: Record<string, number>): FindChart => {
    return async (hash) => (hash in notesByHash ? ({ notes: notesByHash[hash] } as ChartRow) : null)
}

const chartIds = (...hashes: string[]) => hashes.map((hash) => (hash.length === 64 ? { md5: '', sha256: hash } : { md5: hash, sha256: '' }))

describe('sumKnownCourseNotes', () => {
    test('코스의 모든 차트를 서버가 알면 노트 수를 합친다 (sha256 이 없으면 md5 로 찾는다)', async () => {
        const find = createFinder({ [SHA_A]: 1000, [SHA_B]: 1500, [MD5_C]: 700 })
        expect(await sumKnownCourseNotes(chartIds(SHA_A, SHA_B, MD5_C), find)).toBe(3200)
    })

    test('하나라도 모르는 차트가 있으면 0 이라 판정 수에 상한을 두지 않는다', async () => {
        const find = createFinder({ [SHA_A]: 1000 })
        expect(await sumKnownCourseNotes(chartIds(SHA_A, SHA_B), find)).toBe(0)
    })

    test('노트 수가 0 으로 기록된 차트나 빈 목록도 0 이다', async () => {
        expect(await sumKnownCourseNotes(chartIds(SHA_A), createFinder({ [SHA_A]: 0 }))).toBe(0)
        expect(await sumKnownCourseNotes([], createFinder({}))).toBe(0)
    })
})
