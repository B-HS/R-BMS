import { describe, expect, test } from 'bun:test'
import { clearLampToId, gaugeTypeToId } from '@server/dto/common'
import { createDanService, DAN_BACKFILL_PAGE } from '@server/service/domain/dan/dan.service'
import { DAN_AWARD_KIND, DAN_INELIGIBLE_REASON } from '@server/service/domain/dan/dan-eligibility'
import { SCORE_FLAG } from '@server/service/domain/score/ranked-policy'
import { createMemoryDanDb } from '../support/dan-memory-db'
import { DAN_PLAYED_AT, danRun, FIRST_DAN_COURSE, SECOND_DAN_COURSE, STELLA_DAN_COURSE } from '../support/dan-fixture'

const ALICE = 'user-alice'
const BOB = 'user-bob'
const EARLIER_MS = 60_000
const COURSE_NAMES = {
    [FIRST_DAN_COURSE.courseHash]: 'GENOSIDE 2018 段位認定 初段',
    [SECOND_DAN_COURSE.courseHash]: 'GENOSIDE 2018 段位認定 二段',
}

const createService = (seed: Partial<Parameters<typeof createMemoryDanDb>[0]> = {}) => {
    const memory = createMemoryDanDb({ courses: [FIRST_DAN_COURSE, SECOND_DAN_COURSE, STELLA_DAN_COURSE], courseNames: COURSE_NAMES, ...seed })
    return { ...memory, service: createDanService({ db: memory.db }) }
}

const genoside = (mine: Awaited<ReturnType<ReturnType<typeof createService>['service']['listMine']>>) =>
    mine.systems.find((system) => system.system === 'genoside2018_sp')

const SP = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP' } as const
const shownSp = (label: string, assisted = false) => ({ ...SP, label, assisted })
const STELLA_ZERO = { system: 'stella', name: 'Stella', label: 'st0', assisted: false } as const
const AUTO_DISPLAY = { mode: 'auto', label: null, assisted: false } as const

const scratchAutoRun = (course: typeof FIRST_DAN_COURSE, overrides: Parameters<typeof danRun>[1] = {}) =>
    danRun(course, { scratchAuto: true, flags: [SCORE_FLAG.SCRATCH_AUTO], clear: clearLampToId('LightAssistEasy'), ...overrides })

const customJudgeRun = (course: typeof FIRST_DAN_COURSE, overrides: Parameters<typeof danRun>[1] = {}) =>
    danRun(course, { ranked: false, flags: [SCORE_FLAG.ASSIST, SCORE_FLAG.UNKNOWN_BUILD], clear: clearLampToId('AssistEasy'), ...overrides })

describe('awardFromRun', () => {
    test('인정된 판은 단위를 넣고 다시 넣어도 처음 획득이 남는다', async () => {
        const { service, acquired } = createService()
        const first = danRun(FIRST_DAN_COURSE, { id: 'cs_first', userId: ALICE })
        expect(await service.awardFromRun(first)).toEqual({ awarded: FIRST_DAN_COURSE.dan, kind: DAN_AWARD_KIND.FORMAL, reason: null })

        const later = danRun(FIRST_DAN_COURSE, { id: 'cs_later', userId: ALICE, playedAt: DAN_PLAYED_AT + EARLIER_MS, courseGauge: 'IIDX' })
        expect(await service.awardFromRun(later)).toEqual({ awarded: null, kind: DAN_AWARD_KIND.FORMAL, reason: null })

        expect([...acquired.values()]).toEqual([
            {
                userId: ALICE,
                system: 'genoside2018_sp',
                label: '初段',
                assisted: false,
                rank: 1,
                courseHash: FIRST_DAN_COURSE.courseHash,
                courseScoreId: 'cs_first',
                courseGauge: 'LR2',
                gauge: gaugeTypeToId('Class'),
                acquiredAt: DAN_PLAYED_AT,
            },
        ])
    })

    test('단위 코스가 아닌 코스의 판은 not_a_dan_course', async () => {
        const { service, acquired } = createService()
        const result = await service.awardFromRun(danRun(FIRST_DAN_COURSE, { courseHash: 'some-other-course', userId: ALICE }))
        expect(result).toEqual({ awarded: null, kind: null, reason: DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE })
        expect(acquired.size).toBe(0)
    })

    test('인정되지 않는 판은 이유를 돌려주고 아무것도 넣지 않는다', async () => {
        const { service, acquired } = createService()
        const result = await service.awardFromRun(danRun(FIRST_DAN_COURSE, { userId: ALICE, clear: 1 }))
        expect(result).toEqual({ awarded: null, kind: null, reason: DAN_INELIGIBLE_REASON.NOT_CLEARED })
        expect(acquired.size).toBe(0)
    })

    test('어시스트 판은 같은 label 의 A-단위 행을 넣고 정식 행과 따로 남는다', async () => {
        const { service, acquired } = createService()
        expect(await service.awardFromRun(scratchAutoRun(FIRST_DAN_COURSE, { id: 'cs_assisted', userId: ALICE }))).toEqual({
            awarded: FIRST_DAN_COURSE.dan,
            kind: DAN_AWARD_KIND.ASSISTED,
            reason: null,
        })
        expect(await service.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_formal', userId: ALICE }))).toEqual({
            awarded: FIRST_DAN_COURSE.dan,
            kind: DAN_AWARD_KIND.FORMAL,
            reason: null,
        })
        expect(await service.awardFromRun(customJudgeRun(FIRST_DAN_COURSE, { id: 'cs_again', userId: ALICE, rankedWithoutAssist: true }))).toEqual({
            awarded: null,
            kind: DAN_AWARD_KIND.ASSISTED,
            reason: null,
        })
        expect([...acquired.values()].map((row) => [row.label, row.assisted, row.courseScoreId])).toEqual([
            ['初段', true, 'cs_assisted'],
            ['初段', false, 'cs_formal'],
        ])
    })

    test('어시스트만 빼도 순위가 아니었을 넓힌 판정 판은 단위를 주지 않는다', async () => {
        const { service, acquired } = createService()
        const result = await service.awardFromRun(customJudgeRun(FIRST_DAN_COURSE, { userId: ALICE, rankedWithoutAssist: false }))
        expect(result).toEqual({ awarded: null, kind: null, reason: DAN_INELIGIBLE_REASON.UNRANKED })
        expect(acquired.size).toBe(0)
    })
})

describe('표시', () => {
    const awardBoth = async (service: ReturnType<typeof createService>['service']) => {
        await service.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }))
        await service.awardFromRun(danRun(SECOND_DAN_COURSE, { id: 'cs_2', userId: ALICE }))
    }

    test('획득이 없으면 네 체계가 모두 비어 있고 표시는 자동이다', async () => {
        const { service } = createService()
        const mine = await service.listMine(ALICE)
        expect(mine.systems.map((system) => system.system)).toEqual(['genoside2018_sp', 'genoside2018_dp', 'stella', 'satellite'])
        expect(mine.systems.every((system) => system.acquired.length === 0 && system.shown.length === 0 && system.display.mode === 'auto')).toBe(true)
    })

    test('표시 기본은 가장 높은 단위이고 획득 목록은 rank 순에 코스 이름이 붙는다', async () => {
        const { service } = createService()
        await awardBoth(service)
        const system = genoside(await service.listMine(ALICE))
        expect(system?.shown).toEqual([shownSp('二段')])
        expect(system?.display).toEqual(AUTO_DISPLAY)
        expect(
            system?.acquired.map((entry) => [entry.label, entry.assisted, entry.rank, entry.course_name, entry.course_gauge, entry.acquired_at]),
        ).toEqual([
            ['初段', false, 1, 'GENOSIDE 2018 段位認定 初段', 'LR2', DAN_PLAYED_AT],
            ['二段', false, 2, 'GENOSIDE 2018 段位認定 二段', 'LR2', DAN_PLAYED_AT],
        ])
    })

    test('특정 단위를 고르면 그 단위가 보인다', async () => {
        const { service } = createService()
        await awardBoth(service)
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', label: '初段', assisted: false })
        expect(genoside(mine)).toMatchObject({ shown: [shownSp('初段')], display: { mode: 'label', label: '初段', assisted: false } })
        expect((await service.listDisplayed([ALICE])).get(ALICE)).toEqual([shownSp('初段')])
    })

    test('숨김이면 공개 목록에서 빠지고 본인 목록에는 남는다', async () => {
        const { service } = createService()
        await awardBoth(service)
        await service.awardFromRun(danRun(STELLA_DAN_COURSE, { id: 'cs_3', userId: ALICE }))
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'hidden' })
        expect(genoside(mine)).toMatchObject({ shown: [], display: { mode: 'hidden', label: null, assisted: false } })
        expect(genoside(mine)?.acquired).toHaveLength(2)
        expect((await service.listDisplayed([ALICE])).get(ALICE)).toEqual([STELLA_ZERO])
    })

    test('획득하지 않은 label 은 고를 수 없다', async () => {
        const { service, displays } = createService()
        await service.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }))
        await expect(service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', label: '二段', assisted: false })).rejects.toMatchObject({
            code: 'VALIDATION_ERROR',
            details: { field: 'label' },
        })
        await expect(service.setDisplay(BOB, { system: 'genoside2018_sp', mode: 'label', label: '初段', assisted: false })).rejects.toMatchObject({
            code: 'VALIDATION_ERROR',
        })
        expect(displays.size).toBe(0)
    })

    test('자동으로 되돌리면 표시 행이 지워진다', async () => {
        const { service, displays } = createService()
        await awardBoth(service)
        await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'hidden' })
        expect(displays.size).toBe(1)
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'auto' })
        expect(displays.size).toBe(0)
        expect(genoside(mine)).toMatchObject({ shown: [shownSp('二段')], display: AUTO_DISPLAY })
    })

    test('공개 목록은 사용자마다 체계 순서대로 표시 단위만 담고, 획득이 없는 사용자는 빈 목록이다', async () => {
        const { service } = createService()
        await awardBoth(service)
        await service.awardFromRun(danRun(STELLA_DAN_COURSE, { id: 'cs_3', userId: ALICE }))
        await service.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_4', userId: BOB }))
        const displayed = await service.listDisplayed([ALICE, BOB, 'user-nobody'])
        expect(displayed.get(ALICE)).toEqual([shownSp('二段'), STELLA_ZERO])
        expect(displayed.get(BOB)).toEqual([shownSp('初段')])
        expect(displayed.get('user-nobody')).toEqual([])
    })

    test('사용자 목록이 비어 있으면 조회하지 않는다', async () => {
        const { service } = createService()
        expect((await service.listDisplayed([])).size).toBe(0)
    })
})

describe('A-단위 표시', () => {
    const formal = (course: typeof FIRST_DAN_COURSE, id: string) => danRun(course, { id, userId: ALICE })
    const assisted = (course: typeof FIRST_DAN_COURSE, id: string) => scratchAutoRun(course, { id, userId: ALICE })

    const shownAfter = async (runs: ReturnType<typeof danRun>[]) => {
        const { service, displays } = createService()
        for (const run of runs) await service.awardFromRun(run)
        return {
            service,
            displays,
            shown: genoside(await service.listMine(ALICE))?.shown,
            displayed: (await service.listDisplayed([ALICE])).get(ALICE),
        }
    }

    test('자동: 정식만 있으면 정식 하나', async () => {
        const { shown, displayed } = await shownAfter([formal(FIRST_DAN_COURSE, 'cs_1')])
        expect(shown).toEqual([shownSp('初段')])
        expect(displayed).toEqual([shownSp('初段')])
    })

    test('자동: A-단위만 있으면 A- 가 붙은 A-단위 하나', async () => {
        const { shown, displayed } = await shownAfter([assisted(SECOND_DAN_COURSE, 'cs_1')])
        expect(shown).toEqual([shownSp('A-二段', true)])
        expect(displayed).toEqual([shownSp('A-二段', true)])
    })

    test('자동: A-단위가 더 높으면 정식 다음에 A-단위', async () => {
        const { shown, displayed } = await shownAfter([formal(FIRST_DAN_COURSE, 'cs_1'), assisted(SECOND_DAN_COURSE, 'cs_2')])
        expect(shown).toEqual([shownSp('初段'), shownSp('A-二段', true)])
        expect(displayed).toEqual([shownSp('初段'), shownSp('A-二段', true)])
    })

    test('자동: 정식과 A-단위가 같으면 정식만', async () => {
        const { shown } = await shownAfter([assisted(SECOND_DAN_COURSE, 'cs_1'), formal(SECOND_DAN_COURSE, 'cs_2')])
        expect(shown).toEqual([shownSp('二段')])
    })

    test('자동: 정식이 더 높으면 정식만', async () => {
        const { shown } = await shownAfter([assisted(FIRST_DAN_COURSE, 'cs_1'), formal(SECOND_DAN_COURSE, 'cs_2')])
        expect(shown).toEqual([shownSp('二段')])
    })

    test('획득 목록은 rank 순이고 같은 rank 는 정식이 먼저다', async () => {
        const { service } = await shownAfter([
            assisted(SECOND_DAN_COURSE, 'cs_1'),
            formal(SECOND_DAN_COURSE, 'cs_2'),
            assisted(FIRST_DAN_COURSE, 'cs_3'),
        ])
        const acquired = genoside(await service.listMine(ALICE))?.acquired
        expect(acquired?.map((entry) => [entry.label, entry.assisted])).toEqual([
            ['初段', true],
            ['二段', false],
            ['二段', true],
        ])
    })

    test('A-단위를 고르면 그 A-단위 하나만 보이고 같은 label 의 정식은 고르지 않은 것이다', async () => {
        const { service } = await shownAfter([formal(SECOND_DAN_COURSE, 'cs_1'), assisted(SECOND_DAN_COURSE, 'cs_2')])
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', label: '二段', assisted: true })
        expect(genoside(mine)).toMatchObject({ shown: [shownSp('A-二段', true)], display: { mode: 'label', label: '二段', assisted: true } })
        expect((await service.listDisplayed([ALICE])).get(ALICE)).toEqual([shownSp('A-二段', true)])
        const regular = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', label: '二段', assisted: false })
        expect(genoside(regular)?.shown).toEqual([shownSp('二段')])
    })

    test('정식만 가진 단위의 A-단위나 A-단위만 가진 단위의 정식은 고를 수 없다', async () => {
        const { service, displays } = createService()
        await service.awardFromRun(formal(FIRST_DAN_COURSE, 'cs_1'))
        await service.awardFromRun(assisted(SECOND_DAN_COURSE, 'cs_2'))
        for (const choice of [
            { label: '初段', assisted: true },
            { label: '二段', assisted: false },
        ])
            await expect(service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', ...choice })).rejects.toMatchObject({
                code: 'VALIDATION_ERROR',
                details: { field: 'label' },
            })
        expect(displays.size).toBe(0)
    })

    test('숨김은 정식과 A-단위를 모두 감추고 표시 행의 assisted 는 거짓이다', async () => {
        const { service, displays } = await shownAfter([formal(FIRST_DAN_COURSE, 'cs_1'), assisted(SECOND_DAN_COURSE, 'cs_2')])
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'hidden' })
        expect(genoside(mine)).toMatchObject({ shown: [], display: { mode: 'hidden', label: null, assisted: false } })
        expect((await service.listDisplayed([ALICE])).get(ALICE)).toEqual([])
        expect([...displays.values()]).toEqual([{ userId: ALICE, system: 'genoside2018_sp', label: null, assisted: false }])
    })
})

describe('awardForCourses', () => {
    const hashes = [FIRST_DAN_COURSE.courseHash, SECOND_DAN_COURSE.courseHash]
    const ZERO_TOTALS = { scanned: 0, eligible: 0, awarded: 0, eligibleAssisted: 0, awardedAssisted: 0, assistedLegacy: 0 }
    const totals = (counts: Partial<typeof ZERO_TOTALS>) => ({ ...ZERO_TOTALS, ...counts })

    test('소급은 A-단위를 따로 세고, 저장된 사실이 없어 추정한 판을 assistedLegacy 로 센다', async () => {
        const { service, acquired } = createService({
            runs: [
                danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }),
                scratchAutoRun(SECOND_DAN_COURSE, { id: 'cs_2', userId: ALICE }),
                customJudgeRun(SECOND_DAN_COURSE, { id: 'cs_3', userId: BOB, rankedWithoutAssist: null }),
                customJudgeRun(FIRST_DAN_COURSE, { id: 'cs_4', userId: BOB, rankedWithoutAssist: true }),
                customJudgeRun(FIRST_DAN_COURSE, { id: 'cs_5', userId: 'user-carol', rankedWithoutAssist: false }),
            ],
        })
        const expected = totals({ scanned: 5, eligible: 1, eligibleAssisted: 3, assistedLegacy: 1 })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: true })).toEqual(expected)
        expect(acquired.size).toBe(0)
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({ ...expected, awarded: 1, awardedAssisted: 3 })
        expect([...acquired.values()].map((row) => [row.userId, row.label, row.assisted])).toEqual([
            [ALICE, '初段', false],
            [ALICE, '二段', true],
            [BOB, '二段', true],
            [BOB, '初段', true],
        ])
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual(expected)
    })

    test('소급은 stages 가 있는 판만 본다', async () => {
        const { service, acquired } = createService({
            runs: [
                danRun(FIRST_DAN_COURSE, { id: 'cs_with', userId: ALICE }),
                danRun(SECOND_DAN_COURSE, { id: 'cs_without', userId: ALICE, stages: null, gauge: null, courseGauge: null }),
            ],
        })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual(totals({ scanned: 1, eligible: 1, awarded: 1 }))
        expect([...acquired.values()].map((row) => row.label)).toEqual(['初段'])
    })

    test('소급 dryRun 은 넣지 않고 센다', async () => {
        const { service, acquired } = createService({
            runs: [danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }), danRun(FIRST_DAN_COURSE, { id: 'cs_2', userId: ALICE, clear: 1 })],
        })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: true })).toEqual(totals({ scanned: 2, eligible: 1 }))
        expect(acquired.size).toBe(0)
    })

    test('소급은 오래된 판부터 읽어 가장 먼저 친 판의 획득을 남긴다', async () => {
        const { service, acquired } = createService({
            runs: [
                danRun(FIRST_DAN_COURSE, { id: 'cs_a_late', userId: ALICE, playedAt: DAN_PLAYED_AT + EARLIER_MS }),
                danRun(FIRST_DAN_COURSE, { id: 'cs_z_early', userId: ALICE, playedAt: DAN_PLAYED_AT }),
            ],
        })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual(totals({ scanned: 2, eligible: 2, awarded: 1 }))
        expect([...acquired.values()].map((row) => [row.courseScoreId, row.acquiredAt])).toEqual([['cs_z_early', DAN_PLAYED_AT]])
    })

    test('소급을 두 번 돌리면 두 번째는 아무것도 새로 넣지 않는다', async () => {
        const { service } = createService({ runs: [danRun(FIRST_DAN_COURSE, { userId: ALICE })] })
        expect((await service.awardForCourses({ courseHashes: hashes, dryRun: false })).awarded).toBe(1)
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual(totals({ scanned: 1, eligible: 1 }))
    })

    test('단위 코스가 아닌 해시는 건너뛰고 읽지도 않는다', async () => {
        const { service, counts } = createService({ runs: [danRun(FIRST_DAN_COURSE, { userId: ALICE })] })
        expect(await service.awardForCourses({ courseHashes: ['not-a-dan-course'], dryRun: false })).toEqual(totals({}))
        expect(counts.listCourseScoresWithStages).toBe(0)
    })

    test('한 쪽 크기를 넘는 판도 모두 본다', async () => {
        const runs = Array.from({ length: DAN_BACKFILL_PAGE + 1 }, (_, index) =>
            danRun(FIRST_DAN_COURSE, { id: `cs_${String(index).padStart(5, '0')}`, userId: `user-${index}`, playedAt: DAN_PLAYED_AT + index }),
        )
        const { service, counts } = createService({ runs })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual(
            totals({ scanned: DAN_BACKFILL_PAGE + 1, eligible: DAN_BACKFILL_PAGE + 1, awarded: DAN_BACKFILL_PAGE + 1 }),
        )
        expect(counts.listCourseScoresWithStages).toBe(2)
    })

    test('한 쪽이 정확히 찼을 때도 다음 쪽이 비었음을 확인하고 끝난다', async () => {
        const runs = Array.from({ length: DAN_BACKFILL_PAGE }, (_, index) =>
            danRun(FIRST_DAN_COURSE, { id: `cs_${String(index).padStart(5, '0')}`, userId: `user-${index}`, playedAt: DAN_PLAYED_AT + index }),
        )
        const { service, counts } = createService({ runs })
        expect((await service.awardForCourses({ courseHashes: hashes, dryRun: true })).scanned).toBe(DAN_BACKFILL_PAGE)
        expect(counts.listCourseScoresWithStages).toBe(2)
    })

    test('listDanCourseHashes 는 모든 단위 코스를 돌려준다', async () => {
        const { service } = createService()
        expect(await service.listDanCourseHashes()).toEqual([FIRST_DAN_COURSE.courseHash, SECOND_DAN_COURSE.courseHash, STELLA_DAN_COURSE.courseHash])
    })
})
