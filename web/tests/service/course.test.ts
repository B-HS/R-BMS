import { describe, expect, test } from 'bun:test'
import { courseSubmissionSchema } from '@server/dto/course'
import { createCourseService, toCourseMeta, type CourseScoreInsertRow } from '@server/service/domain/course/course.service'

const COURSE_HASH = 'course-hash-1'
const PLAYED_AT = 1_700_000_000_000
const USER = { id: 'user-1', loginId: 'alice' }

const submission = (overrides: Record<string, unknown> = {}) =>
    courseSubmissionSchema.parse({
        course_hash: COURSE_HASH,
        player: { id: 'alice' },
        clear: 'Normal',
        ex_score: 5000,
        judge: { pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 },
        max_combo: 3000,
        gauge_value: 42,
        minbp: 30,
        played_at: PLAYED_AT,
        ...overrides,
    })

const createService = (options: { existingId?: string; best?: { clear: number; exScore: number; minbp: number } } = {}) => {
    const inserted: CourseScoreInsertRow[] = []
    const bests: { courseScoreId: string; clear: number; exScore: number }[] = []
    let counter = 0
    const service = createCourseService({
        newId: (prefix) => `${prefix}${(counter += 1)}`,
        db: {
            findCourse: async () => ({ courseHash: COURSE_HASH, name: '段位', lntype: 1, constraint: ['GRADE'], trophy: [], extra: null }),
            findCourseCharts: async () => [{ position: 0, chartSha256: 'b'.repeat(64), md5: 'c'.repeat(32) }],
            upsertCourse: async () => undefined,
            replaceCourseCharts: async () => undefined,
            findCourseScoreIdByIdempotency: async () => options.existingId ?? null,
            insertCourseScore: async (row) => {
                inserted.push(row)
            },
            getCourseBest: async () => options.best ?? null,
            upsertCourseBest: async (row) => {
                bests.push({ courseScoreId: row.courseScoreId, clear: row.clear, exScore: row.exScore })
            },
            countBetterCourseBests: async () => 2,
            getCourseRanking: async () => [],
            getCourseBestByLoginId: async () => null,
            getRivalUserIds: async () => [],
        },
    })
    return { service, inserted, bests }
}

describe('courseService.submit', () => {
    test('새 제출은 코스 스코어를 넣고 순위를 계산한다', async () => {
        const { service, inserted, bests } = createService()
        const result = await service.submit({ input: submission(), user: USER })
        expect(result.duplicated).toBe(false)
        expect(result.response.rank).toBe(3)
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual([])
        expect(result.response.is_new_best).toBe(true)
        expect(result.response.score_id).toBe(result.courseScoreId)
        expect(inserted[0].courseHash).toBe(COURSE_HASH)
        expect(bests).toHaveLength(1)
    })

    test('같은 (player, course, played_at) 재전송은 기존 id 를 돌려주고 새로 넣지 않는다', async () => {
        const { service, inserted } = createService({ existingId: 'cs_existing' })
        const result = await service.submit({ input: submission(), user: USER })
        expect(result.courseScoreId).toBe('cs_existing')
        expect(result.duplicated).toBe(true)
        expect(inserted).toHaveLength(0)
    })

    test('기존 베스트가 더 좋으면 베스트를 갱신하지 않는다', async () => {
        const { service, bests } = createService({ best: { clear: 6, exScore: 5400, minbp: 10 } })
        await service.submit({ input: submission(), user: USER })
        expect(bests).toHaveLength(0)
    })

    test('인증 제출은 ranked 로 기록된다', async () => {
        const { service, inserted } = createService()
        const result = await service.submit({ input: submission(), user: USER })
        expect(inserted[0].ranked).toBe(true)
        expect(result.response.message).toBe('saved')
    })

    test('played_at 이 음수면 스키마가 거절한다', () => {
        expect(() => submission({ played_at: -1 })).toThrow()
    })

    test('ex_score 가 0 이면 judge 에서 유도한다', async () => {
        const { service, inserted } = createService()
        await service.submit({
            input: submission({
                ex_score: 0,
                judge: { pgreat: 10, great: 4, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0, epg: 6, lpg: 4, egr: 3, lgr: 1 },
            }),
            user: USER,
        })
        expect(inserted[0].exScore).toBe(24)
    })
})

describe('toCourseMeta', () => {
    test('코스 행과 차트 목록을 IR CourseMeta 로 합친다', () => {
        const meta = toCourseMeta({ courseHash: COURSE_HASH, name: '発狂段位', lntype: 1, constraint: ['GRADE'], trophy: null, extra: null }, [
            { position: 0, chartSha256: 'b'.repeat(64), md5: null },
        ])
        expect(meta.name).toBe('発狂段位')
        expect(meta.charts).toEqual([{ md5: '', sha256: 'b'.repeat(64) }])
        expect(meta.trophy).toEqual([])
    })
})
