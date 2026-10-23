import { describe, expect, test } from 'bun:test'
import { clearLampToId, gaugeTypeToId, type AssistGrade } from '@server/dto/common'
import { COURSE_ASSIST_TAG_MAX_LENGTH, COURSE_ASSIST_TAGS_MAX, COURSE_CHARTS_MAX, courseSubmissionSchema } from '@server/dto/course'
import {
    courseAssistGrade,
    createCourseService,
    findCourseIntegrityFailure,
    storedFlagsOf,
    toCourseMeta,
    toStoredStages,
    type CourseScoreInsertRow,
    type StoredCourseRun,
} from '@server/service/domain/course/course.service'
import { BUILD_TRUST, SCORE_FLAG } from '@server/service/domain/score/ranked-policy'

const COURSE_HASH = 'course-hash-1'
const PLAYED_AT = 1_700_000_000_000
const USER = { id: 'user-1', loginId: 'alice' }
const TRUSTED = { buildTrust: BUILD_TRUST.TRUSTED, requireTrustedBuild: false } as const

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

const LOCK_DEADLOCK = Object.assign(new Error('Deadlock found when trying to get lock'), { code: 'ER_LOCK_DEADLOCK' })

const storedRun = (overrides: Partial<StoredCourseRun> & { id: string }): StoredCourseRun => ({
    clear: clearLampToId('Normal'),
    assistGrade: 0,
    exScore: 5000,
    minbp: 30,
    maxCombo: 3000,
    ranked: true,
    scratchAuto: false,
    gauge: null,
    courseGauge: null,
    gaugeValue: 42,
    stages: null,
    playedAt: PLAYED_AT,
    extra: {},
    ...overrides,
})

const CHARTS = [
    { md5: 'a'.repeat(32), sha256: 'b'.repeat(64) },
    { md5: 'c'.repeat(32), sha256: 'd'.repeat(64) },
    { md5: 'e'.repeat(32), sha256: 'f'.repeat(64) },
]
const CLASS_STAGE = { gauge: 'Class', random: 'Off', lntype: 0 } as const
const LIGHT_ASSIST = clearLampToId('LightAssistEasy')
const CLASS_ASSIST_GRADE = 2
const EX_CLASS_ASSIST_GRADE = 3
const EX_HARD_CLASS_ASSIST_GRADE = 4
const EX_HARD_CLASS_ID = gaugeTypeToId('ExHardClass')

const createService = (
    options: {
        existing?: StoredCourseRun
        best?: { clear: number; assistGrade: 0; exScore: number; minbp: number }
        winsBest?: boolean
        remembersInserted?: boolean
        bestFailures?: number
    } = {},
) => {
    const inserted: CourseScoreInsertRow[] = []
    const bests: { courseScoreId: string; clear: number; exScore: number; scratchAuto: boolean }[] = []
    const bestLookups: boolean[] = []
    const rankScopes: boolean[] = []
    const bestGrades: AssistGrade[] = []
    const rankGrades: AssistGrade[] = []
    const calls: string[] = []
    let counter = 0
    const service = createCourseService({
        newId: (prefix) => `${prefix}${(counter += 1)}`,
        db: {
            findCourse: async () => ({ courseHash: COURSE_HASH, name: '段位', lntype: 1, constraint: ['GRADE'], trophy: [], extra: null }),
            findCourseCharts: async () => [{ position: 0, chartSha256: 'b'.repeat(64), md5: 'c'.repeat(32) }],
            upsertCourse: async () => undefined,
            replaceCourseCharts: async () => undefined,
            findCourseScoreByIdempotency: async () => {
                const [row] = inserted
                if (options.remembersInserted && row) return { ...row, assistGrade: courseAssistGrade(row) }
                return options.existing ? { ...options.existing, assistGrade: courseAssistGrade(options.existing) } : null
            },
            insertCourseScore: async (row) => {
                inserted.push(row)
            },
            getCourseBest: async ({ scratchAuto }) => {
                bestLookups.push(scratchAuto)
                return options.best ?? null
            },
            upsertCourseBestIfBetter: async (row) => {
                calls.push('best')
                bestGrades.push(row.assistGrade)
                if (calls.filter((call) => call === 'best').length <= (options.bestFailures ?? 0)) throw LOCK_DEADLOCK
                bests.push({ courseScoreId: row.courseScoreId, clear: row.clear, exScore: row.exScore, scratchAuto: row.scratchAuto })
                return options.winsBest ?? true
            },
            countBetterCourseBests: async ({ includeScratchAuto, assistGrade }) => {
                calls.push('rank')
                rankScopes.push(includeScratchAuto)
                rankGrades.push(assistGrade)
                return 2
            },
            getCourseRanking: async () => [],
            getCourseBestByLoginId: async () => null,
            getRivalUserIds: async () => [],
        },
    })
    return { service, inserted, bests, bestLookups, rankScopes, bestGrades, rankGrades, calls }
}

describe('courseService.submit', () => {
    test('새 제출은 코스 스코어를 넣고 순위를 계산한다', async () => {
        const { service, inserted, bests } = createService()
        const result = await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(result.duplicated).toBe(false)
        expect(result.response.rank).toBe(3)
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual([])
        expect(result.response.is_new_best).toBe(true)
        expect(result.response.score_id).toBe(result.courseScoreId)
        expect(inserted[0].courseHash).toBe(COURSE_HASH)
        expect(bests).toHaveLength(1)
    })

    test('같은 (player, course, played_at) 재전송은 기존 id 를 돌려주고 새로 넣지 않으며, 저장된 판의 값으로 베스트를 맞춘 뒤 순위를 센다', async () => {
        const { service, inserted, bests, calls } = createService({ existing: storedRun({ id: 'cs_existing', exScore: 4800 }) })
        const result = await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(result.courseScoreId).toBe('cs_existing')
        expect(result.duplicated).toBe(true)
        expect(inserted).toHaveLength(0)
        expect(bests).toEqual([{ courseScoreId: 'cs_existing', clear: clearLampToId('Normal'), exScore: 4800, scratchAuto: false }])
        expect(calls).toEqual(['best', 'rank'])
        expect(result.response.rank).toBe(3)
        expect(result.response.is_new_best).toBe(false)
    })

    test('저장된 코스 판이 unranked 면 재전송은 베스트를 건드리지 않는다', async () => {
        const { service, bests } = createService({ existing: storedRun({ id: 'cs_unranked', ranked: false }) })
        const result = await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(bests).toHaveLength(0)
        expect(result.response.rank).toBeNull()
    })

    test('코스 점수는 남고 베스트 갱신이 실패한 판을 다시 보내면 베스트가 그 판으로 바뀐다', async () => {
        const { service, inserted, bests } = createService({ remembersInserted: true, bestFailures: 1 })
        await expect(service.submit({ input: submission(), user: USER, ...TRUSTED })).rejects.toBe(LOCK_DEADLOCK)
        expect(inserted).toHaveLength(1)
        const resent = await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(resent.duplicated).toBe(true)
        expect(bests).toEqual([{ courseScoreId: inserted[0].id, clear: clearLampToId('Normal'), exScore: 5000, scratchAuto: false }])
    })

    test('기존 베스트가 더 좋으면 베스트를 갱신하지 않는다', async () => {
        const { service, bests } = createService({ best: { clear: 6, assistGrade: 0, exScore: 5400, minbp: 10 } })
        await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(bests).toHaveLength(0)
    })

    test('FAILED 코스는 단일 곡 실패 판과 같이 저장하고 ranked 로 순위와 베스트 경쟁에 넣는다', async () => {
        const { service, inserted, bests } = createService()
        const result = await service.submit({ input: submission({ clear: 'Failed', ex_score: 1200, minbp: 900 }), user: USER, ...TRUSTED })
        expect(result.response.accepted).toBe(true)
        expect(result.response.ranked).toBe(true)
        expect(result.response.rank).toBe(3)
        expect(inserted[0].clear).toBe(clearLampToId('Failed'))
        expect(bests).toHaveLength(1)
    })

    test('FAILED 코스는 더 높은 램프의 최고 기록을 밀어내지 않는다', async () => {
        const { service, bests } = createService({ best: { clear: clearLampToId('Normal'), assistGrade: 0, exScore: 100, minbp: 999 } })
        const result = await service.submit({ input: submission({ clear: 'Failed', ex_score: 4000, minbp: 10 }), user: USER, ...TRUSTED })
        expect(result.response.is_new_best).toBe(false)
        expect(bests).toHaveLength(0)
    })

    test('인증 제출은 ranked 로 기록된다', async () => {
        const { service, inserted } = createService()
        const result = await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(inserted[0].ranked).toBe(true)
        expect(result.response.message).toBe('saved')
    })

    test('어시스트 필드가 없는 옛 클라이언트 제출은 어시스트 없는 코스로 받아 전과 같이 저장한다', async () => {
        const input = submission()
        expect(input.scratch_auto).toBe(false)
        expect(input.assist).toEqual([])
        const { service, inserted, bests } = createService()
        const result = await service.submit({ input, user: USER, ...TRUSTED })
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
            ...TRUSTED,
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
        await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(inserted[0].scratchAuto).toBe(false)
        expect(bests[0].scratchAuto).toBe(false)
        expect(bestLookups).toEqual([false])
        expect(rankScopes).toEqual([false])
    })

    test('넓힌 판정 스테이지가 낀 코스는 ASSIST 로 unranked 다', async () => {
        const { service, inserted } = createService()
        const result = await service.submit({ input: submission({ assist: ['CUSTOM_JUDGE'] }), user: USER, ...TRUSTED })
        expect(result.response.ranked).toBe(false)
        expect(result.response.flags).toEqual(['ASSIST'])
        expect(inserted[0].extra).toEqual({ flags: ['ASSIST'], scratch_auto: false, assist: ['CUSTOM_JUDGE'] })
    })

    test('SCRATCH AUTO 와 넓힌 판정이 함께 낀 코스는 unranked 이고 베스트에 넣지 않는다', async () => {
        const { service, bests } = createService()
        const result = await service.submit({
            input: submission({ scratch_auto: true, assist: ['AUTO_SCRATCH', 'CUSTOM_JUDGE'] }),
            user: USER,
            ...TRUSTED,
        })
        expect(result.response.ranked).toBe(false)
        expect(result.response.flags).toEqual(['SCRATCH_AUTO', 'ASSIST'])
        expect(result.response.rank).toBeNull()
        expect(result.response.message).toBe('recorded (unranked: assist)')
        expect(bests).toHaveLength(0)
    })

    test('SCRATCH AUTO 코스의 재전송도 ranked 와 flags 를 그대로 돌려준다', async () => {
        const { service } = createService({ existing: storedRun({ id: 'cs_existing', scratchAuto: true }) })
        const result = await service.submit({ input: submission({ scratch_auto: true, assist: ['AUTO_SCRATCH'] }), user: USER, ...TRUSTED })
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
            ...TRUSTED,
        })
        expect(inserted[0].exScore).toBe(24)
    })
})

describe('코스 게이지와 어시스트 등급 (결정 6)', () => {
    const scratchAutoClear = { clear: 'LightAssistEasy', scratch_auto: true, assist: ['AUTO_SCRATCH'] }

    test('게이지가 있는 SCRATCH AUTO 코스 클리어는 그 게이지 등급으로 최고 기록과 순위에 들어간다', async () => {
        const { service, inserted, bestGrades, rankGrades } = createService()
        await service.submit({ input: submission({ ...scratchAutoClear, gauge: 'ExClass' }), user: USER, ...TRUSTED })
        expect(inserted[0].gauge).toBe(gaugeTypeToId('ExClass'))
        expect(bestGrades).toEqual([EX_CLASS_ASSIST_GRADE])
        expect(rankGrades).toEqual([EX_CLASS_ASSIST_GRADE])
    })

    test('게이지가 없는 옛 코스 본문은 등급 0 이고 게이지와 스테이지는 비어 있다', async () => {
        const input = submission(scratchAutoClear)
        expect(input.gauge).toBeNull()
        expect(input.course_gauge).toBeNull()
        expect(input.stages).toEqual([])
        const { service, inserted, bestGrades } = createService()
        await service.submit({ input, user: USER, ...TRUSTED })
        expect(bestGrades).toEqual([0])
        expect([inserted[0].gauge, inserted[0].courseGauge, inserted[0].stages]).toEqual([null, null, null])
    })

    test('LIGHT ASSIST 램프가 아니면 게이지가 있어도 등급은 0 이다', async () => {
        const { service, bestGrades } = createService()
        await service.submit({ input: submission({ clear: 'Hard', gauge: 'ExHardClass' }), user: USER, ...TRUSTED })
        expect(bestGrades).toEqual([0])
    })

    test('스테이지는 같은 자리 차트와 짝지어 저장된다', async () => {
        const { service, inserted } = createService()
        const stages = [CLASS_STAGE, { ...CLASS_STAGE, random: 'Mirror', lntype: 1 }]
        await service.submit({ input: submission({ charts: CHARTS, stages }), user: USER, ...TRUSTED })
        expect(inserted[0].stages).toHaveLength(2)
        expect(inserted[0].stages?.[1]).toEqual({ gauge: 'Class', random: 'Mirror', lntype: 1, md5: CHARTS[1].md5, sha256: CHARTS[1].sha256 })
    })

    test('차트보다 많은 스테이지는 차트가 없는 자리를 빈 해시로 저장한다', () => {
        expect(toStoredStages({ charts: [CHARTS[0]], stages: [CLASS_STAGE, CLASS_STAGE] })).toEqual([
            { ...CLASS_STAGE, md5: CHARTS[0].md5, sha256: CHARTS[0].sha256 },
            { ...CLASS_STAGE, md5: '', sha256: '' },
        ])
    })

    test('코스 게이지 모드는 그대로 저장된다', async () => {
        const { service, inserted } = createService()
        await service.submit({ input: submission({ course_gauge: 'IIDX' }), user: USER, ...TRUSTED })
        expect(inserted[0].courseGauge).toBe('IIDX')
    })

    test('같은 판을 다시 보내면 처음 판에 저장된 게이지로 등급을 다시 만든다', async () => {
        const existing = storedRun({ id: 'cs_existing', clear: LIGHT_ASSIST, gauge: EX_HARD_CLASS_ID, scratchAuto: true })
        const { service, bestGrades, rankGrades } = createService({ existing })
        await service.submit({ input: submission(scratchAutoClear), user: USER, ...TRUSTED })
        expect(bestGrades).toEqual([EX_HARD_CLASS_ASSIST_GRADE])
        expect(rankGrades).toEqual([EX_HARD_CLASS_ASSIST_GRADE])
    })

    test('게이지를 모르는 옛 행을 다시 보내면 다시 보낸 본문에 게이지가 있어도 등급은 0 이다', async () => {
        const { service, bestGrades } = createService({ existing: storedRun({ id: 'cs_old', clear: LIGHT_ASSIST, scratchAuto: true }) })
        await service.submit({ input: submission({ ...scratchAutoClear, gauge: 'ExHardClass' }), user: USER, ...TRUSTED })
        expect(bestGrades).toEqual([0])
    })

    test('범위 밖 lntype 도 판을 거절하지 않는다', () => {
        expect(submission({ stages: [{ ...CLASS_STAGE, lntype: 7 }] }).stages[0].lntype).toBe(7)
    })

    test('알 수 없는 코스 게이지 이름이나 차트 한도를 넘는 스테이지 수는 스키마가 거절한다', () => {
        expect(() => submission({ course_gauge: 'LR1' })).toThrow()
        expect(() => submission({ stages: Array.from({ length: COURSE_CHARTS_MAX + 1 }, () => CLASS_STAGE) })).toThrow()
        expect(() => submission({ stages: Array.from({ length: COURSE_CHARTS_MAX }, () => CLASS_STAGE) })).not.toThrow()
    })

    test('새 판의 scoreRow 는 저장한 행의 값과 이번 제출의 표지를 싣는다', async () => {
        const { service, inserted } = createService()
        const result = await service.submit({
            input: submission({ ...scratchAutoClear, charts: CHARTS, gauge: 'Class', course_gauge: 'LR2', stages: [CLASS_STAGE] }),
            user: USER,
            ...TRUSTED,
        })
        expect(result.scoreRow).toEqual({
            id: result.courseScoreId,
            userId: USER.id,
            courseHash: COURSE_HASH,
            clear: LIGHT_ASSIST,
            ranked: true,
            scratchAuto: true,
            flags: [SCORE_FLAG.SCRATCH_AUTO],
            gauge: gaugeTypeToId('Class'),
            courseGauge: 'LR2',
            gaugeValue: 42,
            stages: [{ ...CLASS_STAGE, md5: CHARTS[0].md5, sha256: CHARTS[0].sha256 }],
            playedAt: PLAYED_AT,
        })
        expect(result.scoreRow.stages).toBe(inserted[0].stages)
    })

    test('다시 보낸 판의 scoreRow 는 다시 보낸 본문이 아니라 저장된 행에서 온다', async () => {
        const stored = storedRun({
            id: 'cs_existing',
            clear: LIGHT_ASSIST,
            scratchAuto: true,
            gauge: gaugeTypeToId('ExClass'),
            courseGauge: 'BEATORAJA',
            gaugeValue: 17,
            stages: [{ ...CLASS_STAGE, md5: CHARTS[0].md5, sha256: CHARTS[0].sha256 }],
            playedAt: PLAYED_AT + 1,
            extra: { flags: [SCORE_FLAG.SCRATCH_AUTO] },
        })
        const { service, inserted } = createService({ existing: stored })
        const result = await service.submit({ input: submission({ gauge: 'Easy' }), user: USER, ...TRUSTED })
        expect(inserted).toHaveLength(0)
        expect(result.scoreRow).toEqual({
            id: 'cs_existing',
            userId: USER.id,
            courseHash: COURSE_HASH,
            clear: LIGHT_ASSIST,
            ranked: true,
            scratchAuto: true,
            flags: [SCORE_FLAG.SCRATCH_AUTO],
            gauge: gaugeTypeToId('ExClass'),
            courseGauge: 'BEATORAJA',
            gaugeValue: 17,
            stages: stored.stages,
            playedAt: PLAYED_AT + 1,
        })
    })
})

describe('courseAssistGrade', () => {
    test('LIGHT ASSIST 램프만 게이지 등급을 받고 게이지를 모르면 0 이다', () => {
        expect(courseAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('Class') })).toBe(CLASS_ASSIST_GRADE)
        expect(courseAssistGrade({ clear: LIGHT_ASSIST, gauge: gaugeTypeToId('ExClass') })).toBe(EX_CLASS_ASSIST_GRADE)
        expect(courseAssistGrade({ clear: LIGHT_ASSIST, gauge: EX_HARD_CLASS_ID })).toBe(EX_HARD_CLASS_ASSIST_GRADE)
        expect(courseAssistGrade({ clear: LIGHT_ASSIST, gauge: null })).toBe(0)
        expect(courseAssistGrade({ clear: clearLampToId('Normal'), gauge: EX_HARD_CLASS_ID })).toBe(0)
    })
})

describe('storedFlagsOf', () => {
    test('저장된 extra.flags 에서 알려진 표지만 남긴다', () => {
        expect(storedFlagsOf({ flags: [SCORE_FLAG.ASSIST, 'NOT_A_FLAG', 3], scratch_auto: false })).toEqual([SCORE_FLAG.ASSIST])
        expect(storedFlagsOf({ flags: 'ASSIST' })).toEqual([])
        expect(storedFlagsOf({})).toEqual([])
        expect(storedFlagsOf(null)).toEqual([])
    })
})

describe('courseService.submit 의 경합과 빌드 정책 (IR-1, IR-4)', () => {
    test('DB 가 더 좋은 코스 최고 기록이 이미 있다고 답하면 is_new_best 는 false 다', async () => {
        const { service } = createService({ winsBest: false })
        const result = await service.submit({ input: submission(), user: USER, ...TRUSTED })
        expect(result.response.is_new_best).toBe(false)
        expect(result.response.ranked).toBe(true)
    })

    test('미등록 빌드는 단일 곡과 같이 UNKNOWN_BUILD 표지만 남기고 기본 설정에서는 ranked 다', async () => {
        const { service, bests } = createService()
        const result = await service.submit({ input: submission(), user: USER, buildTrust: BUILD_TRUST.UNKNOWN, requireTrustedBuild: false })
        expect(result.response.flags).toEqual(['UNKNOWN_BUILD'])
        expect(result.response.ranked).toBe(true)
        expect(bests).toHaveLength(1)
    })

    test('REQUIRE_BUILD_HASH 가 켜지면 미등록 빌드의 코스는 unranked 이고 베스트에 넣지 않는다', async () => {
        const { service, bests, inserted } = createService()
        const result = await service.submit({ input: submission(), user: USER, buildTrust: BUILD_TRUST.UNKNOWN, requireTrustedBuild: true })
        expect(result.response.ranked).toBe(false)
        expect(result.response.rank).toBeNull()
        expect(inserted[0].ranked).toBe(false)
        expect(bests).toHaveLength(0)
    })

    test('차단된 빌드의 코스는 unranked 다', async () => {
        const { service, bests } = createService()
        const result = await service.submit({ input: submission(), user: USER, buildTrust: BUILD_TRUST.UNTRUSTED, requireTrustedBuild: false })
        expect(result.response.ranked).toBe(false)
        expect(bests).toHaveLength(0)
    })
})

describe('findCourseIntegrityFailure', () => {
    const judge = { pgreat: 600, great: 100, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0 }
    const parsed = (overrides: Record<string, unknown>) => submission({ judge, ex_score: 1300, max_ex_score: 1400, minbp: 0, ...overrides })

    test('판정이 만든 EX 와 같고 노트 수 안이면 통과한다', () => {
        expect(findCourseIntegrityFailure(parsed({}))).toBeNull()
    })

    test('EX 가 판정과 어긋나면 ex_score 로 거절한다', () => {
        expect(findCourseIntegrityFailure(parsed({ ex_score: 2147483647 }))).toEqual({
            field: 'ex_score',
            reason: 'ex_score does not match the judge counts',
        })
    })

    test('노트를 가져간 판정은 본문의 max_ex_score(노트당 2)를 넘을 수 없다 — 차트 목록을 비워도 마찬가지다 (R2-ACC-1)', () => {
        expect(findCourseIntegrityFailure(parsed({ max_ex_score: 1399 }))).toEqual({
            field: 'judge',
            reason: 'judge count exceeds the notes of the course',
        })
        const forged = submission({ judge: { ...judge, pgreat: 1_000_000, great: 0 }, ex_score: 2_000_000, charts: [] })
        expect(findCourseIntegrityFailure(forged)).toEqual({ field: 'judge', reason: 'judge count exceeds the notes of the course' })
    })

    test("코스 본문은 모드를 모르므로 노트를 남기는 pop'n BAD 는 세지 않는다", () => {
        expect(findCourseIntegrityFailure(parsed({ judge: { ...judge, bad: 50 }, minbp: 50 }))).toBeNull()
    })

    test('minbp 는 0 이거나 BAD+POOR+空POOR 이상이다', () => {
        const judged = { ...judge, pgreat: 500, poor: 100 }
        expect(findCourseIntegrityFailure(parsed({ judge: judged, ex_score: 1100, minbp: 1 }))).toEqual({
            field: 'minbp',
            reason: 'minbp is below the BAD, POOR and empty POOR count',
        })
        expect(findCourseIntegrityFailure(parsed({ judge: judged, ex_score: 1100, minbp: 100 }))).toBeNull()
    })

    test('0.1.0 이 보내는 마지막 곡의 램프는 코스 판정과 맞지 않아도 받는다', () => {
        expect(findCourseIntegrityFailure(parsed({ judge: { ...judge, poor: 5 }, max_ex_score: 1410, clear: 'Max', minbp: 5 }))).toBeNull()
    })
})

describe('코스 판정 합계 저장 (R2-ACC-3)', () => {
    test('클라이언트가 보낸 합계·FAST/SLOW·콤보 끊김·空POOR·최대 EX 를 그대로 저장한다', async () => {
        const { service, inserted } = createService()
        const judge = { pgreat: 56, great: 3, good: 1, bad: 1, poor: 2, miss: 4, fast: 40, slow: 20, combobreak: 3, empty_poor: 4 }
        await service.submit({ input: submission({ judge, ex_score: 115, max_ex_score: 126, minbp: 7 }), user: USER, ...TRUSTED })
        expect(inserted[0]).toMatchObject({
            pgreat: 56,
            great: 3,
            good: 1,
            bad: 1,
            poor: 2,
            miss: 4,
            fast: 40,
            slow: 20,
            combobreak: 3,
            emptyPoor: 4,
            maxExScore: 126,
            epg: 0,
            lpg: 0,
        })
    })
})

describe('courseSubmissionSchema 의 한도', () => {
    test('차트 목록이 한도를 넘으면 거절한다', () => {
        expect(() =>
            submission({ charts: Array.from({ length: COURSE_CHARTS_MAX + 1 }, () => ({ md5: 'a'.repeat(32), sha256: 'b'.repeat(64) })) }),
        ).toThrow()
        expect(() =>
            submission({ charts: Array.from({ length: COURSE_CHARTS_MAX }, () => ({ md5: 'a'.repeat(32), sha256: 'b'.repeat(64) })) }),
        ).not.toThrow()
    })

    test('빌드 해시와 플랫폼은 선택 필드라 옛 본문도 그대로 받는다', () => {
        const input = submission()
        expect(input.client_build_sha256).toBeNull()
        expect(input.client_platform).toBeNull()
        expect(submission({ client_build_sha256: 'c'.repeat(64), client_platform: 'macos' }).client_build_sha256).toBe('c'.repeat(64))
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
