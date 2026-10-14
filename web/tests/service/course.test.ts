import { describe, expect, test } from 'bun:test'
import { COURSE_ASSIST_TAG_MAX_LENGTH, COURSE_ASSIST_TAGS_MAX, courseSubmissionSchema } from '@server/dto/course'
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
    const bests: { courseScoreId: string; clear: number; exScore: number; scratchAuto: boolean }[] = []
    const bestLookups: boolean[] = []
    const rankScopes: boolean[] = []
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
            getCourseBest: async ({ scratchAuto }) => {
                bestLookups.push(scratchAuto)
                return options.best ?? null
            },
            upsertCourseBest: async (row) => {
                bests.push({ courseScoreId: row.courseScoreId, clear: row.clear, exScore: row.exScore, scratchAuto: row.scratchAuto })
            },
            countBetterCourseBests: async ({ includeScratchAuto }) => {
                rankScopes.push(includeScratchAuto)
                return 2
            },
            getCourseRanking: async () => [],
            getCourseBestByLoginId: async () => null,
            getRivalUserIds: async () => [],
        },
    })
    return { service, inserted, bests, bestLookups, rankScopes }
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

    test('어시스트 필드가 없는 옛 클라이언트 제출은 어시스트 없는 코스로 받아 전과 같이 저장한다', async () => {
        const input = submission()
        expect(input.scratch_auto).toBe(false)
        expect(input.assist).toEqual([])
        const { service, inserted, bests } = createService()
        const result = await service.submit({ input, user: USER })
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual([])
        expect(inserted[0].extra).toEqual({})
        expect(bests).toHaveLength(1)
    })

    test('SCRATCH AUTO 스테이지가 낀 코스는 ranked 로 저장하고 SCRATCH AUTO 베스트와 포함 목록 순위에 넣는다', async () => {
        const { service, inserted, bests, bestLookups, rankScopes } = createService()
        const result = await service.submit({
            input: submission({ scratch_auto: true, assist: ['AUTO_SCRATCH'], extra: { note: 'kept' } }),
            user: USER,
        })
        expect(result.duplicated).toBe(false)
        expect(result.response.accepted).toBe(true)
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual(['SCRATCH_AUTO'])
        expect(result.response.rank).toBe(3)
        expect(result.response.is_new_best).toBe(true)
        expect(result.response.message).toBe('saved (scratch auto)')
        expect(inserted[0].ranked).toBe(true)
        expect(inserted[0].scratchAuto).toBe(true)
        expect(inserted[0].extra).toEqual({ note: 'kept', flags: ['SCRATCH_AUTO'], scratch_auto: true, assist: ['AUTO_SCRATCH'] })
        expect(bests).toEqual([{ courseScoreId: result.courseScoreId, clear: 5, exScore: 5000, scratchAuto: true }])
        expect(bestLookups).toEqual([true])
        expect(rankScopes).toEqual([true])
    })

    test('일반 코스 판은 일반 베스트와 기본 목록 순위를 쓴다', async () => {
        const { service, inserted, bests, bestLookups, rankScopes } = createService()
        await service.submit({ input: submission(), user: USER })
        expect(inserted[0].scratchAuto).toBe(false)
        expect(bests[0].scratchAuto).toBe(false)
        expect(bestLookups).toEqual([false])
        expect(rankScopes).toEqual([false])
    })

    test('넓힌 판정 스테이지가 낀 코스는 ASSIST 로 unranked 다', async () => {
        const { service, inserted } = createService()
        const result = await service.submit({ input: submission({ assist: ['CUSTOM_JUDGE'] }), user: USER })
        expect(result.response.ranked).toBe(false)
        expect(result.response.flags).toEqual(['ASSIST'])
        expect(inserted[0].extra).toEqual({ flags: ['ASSIST'], scratch_auto: false, assist: ['CUSTOM_JUDGE'] })
    })

    test('SCRATCH AUTO 와 넓힌 판정이 함께 낀 코스는 unranked 이고 베스트에 넣지 않는다', async () => {
        const { service, bests } = createService()
        const result = await service.submit({ input: submission({ scratch_auto: true, assist: ['AUTO_SCRATCH', 'CUSTOM_JUDGE'] }), user: USER })
        expect(result.response.ranked).toBe(false)
        expect(result.response.flags).toEqual(['SCRATCH_AUTO', 'ASSIST'])
        expect(result.response.rank).toBeNull()
        expect(result.response.message).toBe('recorded (unranked: assist)')
        expect(bests).toHaveLength(0)
    })

    test('SCRATCH AUTO 코스의 재전송도 ranked 와 flags 를 그대로 돌려준다', async () => {
        const { service } = createService({ existingId: 'cs_existing' })
        const result = await service.submit({ input: submission({ scratch_auto: true, assist: ['AUTO_SCRATCH'] }), user: USER })
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual(['SCRATCH_AUTO'])
    })

    test('어시스트 태그가 지나치게 많거나 길면 스키마가 거절한다', () => {
        expect(() => submission({ assist: Array.from({ length: COURSE_ASSIST_TAGS_MAX + 1 }, () => 'AUTO_SCRATCH') })).toThrow()
        expect(() => submission({ assist: ['X'.repeat(COURSE_ASSIST_TAG_MAX_LENGTH + 1)] })).toThrow()
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
