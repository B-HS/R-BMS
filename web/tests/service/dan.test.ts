import { describe, expect, test } from 'bun:test'
import { gaugeTypeToId } from '@server/dto/common'
import { createDanService, DAN_BACKFILL_PAGE } from '@server/service/domain/dan/dan.service'
import { DAN_INELIGIBLE_REASON } from '@server/service/domain/dan/dan-eligibility'
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

describe('awardFromRun', () => {
    test('인정된 판은 단위를 넣고 다시 넣어도 처음 획득이 남는다', async () => {
        const { service, acquired } = createService()
        const first = danRun(FIRST_DAN_COURSE, { id: 'cs_first', userId: ALICE })
        expect(await service.awardFromRun(first)).toEqual({ awarded: FIRST_DAN_COURSE.dan, reason: null })

        const later = danRun(FIRST_DAN_COURSE, { id: 'cs_later', userId: ALICE, playedAt: DAN_PLAYED_AT + EARLIER_MS, courseGauge: 'IIDX' })
        expect(await service.awardFromRun(later)).toEqual({ awarded: null, reason: null })

        expect([...acquired.values()]).toEqual([
            {
                userId: ALICE,
                system: 'genoside2018_sp',
                label: '初段',
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
        expect(result).toEqual({ awarded: null, reason: DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE })
        expect(acquired.size).toBe(0)
    })

    test('인정되지 않는 판은 이유를 돌려주고 아무것도 넣지 않는다', async () => {
        const { service, acquired } = createService()
        const result = await service.awardFromRun(danRun(FIRST_DAN_COURSE, { userId: ALICE, clear: 1 }))
        expect(result).toEqual({ awarded: null, reason: DAN_INELIGIBLE_REASON.NOT_CLEARED })
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
        expect(mine.systems.every((system) => system.acquired.length === 0 && system.shown === null && system.display.mode === 'auto')).toBe(true)
    })

    test('표시 기본은 가장 높은 단위이고 획득 목록은 rank 순에 코스 이름이 붙는다', async () => {
        const { service } = createService()
        await awardBoth(service)
        const system = genoside(await service.listMine(ALICE))
        expect(system?.shown).toBe('二段')
        expect(system?.display).toEqual({ mode: 'auto', label: null })
        expect(system?.acquired.map((entry) => [entry.label, entry.rank, entry.course_name, entry.course_gauge, entry.acquired_at])).toEqual([
            ['初段', 1, 'GENOSIDE 2018 段位認定 初段', 'LR2', DAN_PLAYED_AT],
            ['二段', 2, 'GENOSIDE 2018 段位認定 二段', 'LR2', DAN_PLAYED_AT],
        ])
    })

    test('특정 단위를 고르면 그 단위가 보인다', async () => {
        const { service } = createService()
        await awardBoth(service)
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', label: '初段' })
        expect(genoside(mine)).toMatchObject({ shown: '初段', display: { mode: 'label', label: '初段' } })
        expect((await service.listDisplayed([ALICE])).get(ALICE)).toEqual([{ system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '初段' }])
    })

    test('숨김이면 공개 목록에서 빠지고 본인 목록에는 남는다', async () => {
        const { service } = createService()
        await awardBoth(service)
        await service.awardFromRun(danRun(STELLA_DAN_COURSE, { id: 'cs_3', userId: ALICE }))
        const mine = await service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'hidden' })
        expect(genoside(mine)).toMatchObject({ shown: null, display: { mode: 'hidden', label: null } })
        expect(genoside(mine)?.acquired).toHaveLength(2)
        expect((await service.listDisplayed([ALICE])).get(ALICE)).toEqual([{ system: 'stella', name: 'Stella', label: 'st0' }])
    })

    test('획득하지 않은 label 은 고를 수 없다', async () => {
        const { service, displays } = createService()
        await service.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }))
        await expect(service.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'label', label: '二段' })).rejects.toMatchObject({
            code: 'VALIDATION_ERROR',
            details: { field: 'label' },
        })
        await expect(service.setDisplay(BOB, { system: 'genoside2018_sp', mode: 'label', label: '初段' })).rejects.toMatchObject({
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
        expect(genoside(mine)).toMatchObject({ shown: '二段', display: { mode: 'auto' } })
    })

    test('공개 목록은 사용자마다 체계 순서대로 표시 단위만 담고, 획득이 없는 사용자는 빈 목록이다', async () => {
        const { service } = createService()
        await awardBoth(service)
        await service.awardFromRun(danRun(STELLA_DAN_COURSE, { id: 'cs_3', userId: ALICE }))
        await service.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_4', userId: BOB }))
        const displayed = await service.listDisplayed([ALICE, BOB, 'user-nobody'])
        expect(displayed.get(ALICE)).toEqual([
            { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '二段' },
            { system: 'stella', name: 'Stella', label: 'st0' },
        ])
        expect(displayed.get(BOB)).toEqual([{ system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '初段' }])
        expect(displayed.get('user-nobody')).toEqual([])
    })

    test('사용자 목록이 비어 있으면 조회하지 않는다', async () => {
        const { service } = createService()
        expect((await service.listDisplayed([])).size).toBe(0)
    })
})

describe('awardForCourses', () => {
    const hashes = [FIRST_DAN_COURSE.courseHash, SECOND_DAN_COURSE.courseHash]

    test('소급은 stages 가 있는 판만 본다', async () => {
        const { service, acquired } = createService({
            runs: [
                danRun(FIRST_DAN_COURSE, { id: 'cs_with', userId: ALICE }),
                danRun(SECOND_DAN_COURSE, { id: 'cs_without', userId: ALICE, stages: null, gauge: null, courseGauge: null }),
            ],
        })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({ scanned: 1, eligible: 1, awarded: 1 })
        expect([...acquired.values()].map((row) => row.label)).toEqual(['初段'])
    })

    test('소급 dryRun 은 넣지 않고 센다', async () => {
        const { service, acquired } = createService({
            runs: [danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }), danRun(FIRST_DAN_COURSE, { id: 'cs_2', userId: ALICE, clear: 1 })],
        })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: true })).toEqual({ scanned: 2, eligible: 1, awarded: 0 })
        expect(acquired.size).toBe(0)
    })

    test('소급은 오래된 판부터 읽어 가장 먼저 친 판의 획득을 남긴다', async () => {
        const { service, acquired } = createService({
            runs: [
                danRun(FIRST_DAN_COURSE, { id: 'cs_a_late', userId: ALICE, playedAt: DAN_PLAYED_AT + EARLIER_MS }),
                danRun(FIRST_DAN_COURSE, { id: 'cs_z_early', userId: ALICE, playedAt: DAN_PLAYED_AT }),
            ],
        })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({ scanned: 2, eligible: 2, awarded: 1 })
        expect([...acquired.values()].map((row) => [row.courseScoreId, row.acquiredAt])).toEqual([['cs_z_early', DAN_PLAYED_AT]])
    })

    test('소급을 두 번 돌리면 두 번째는 아무것도 새로 넣지 않는다', async () => {
        const { service } = createService({ runs: [danRun(FIRST_DAN_COURSE, { userId: ALICE })] })
        expect((await service.awardForCourses({ courseHashes: hashes, dryRun: false })).awarded).toBe(1)
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({ scanned: 1, eligible: 1, awarded: 0 })
    })

    test('단위 코스가 아닌 해시는 건너뛰고 읽지도 않는다', async () => {
        const { service, counts } = createService({ runs: [danRun(FIRST_DAN_COURSE, { userId: ALICE })] })
        expect(await service.awardForCourses({ courseHashes: ['not-a-dan-course'], dryRun: false })).toEqual({ scanned: 0, eligible: 0, awarded: 0 })
        expect(counts.listCourseScoresWithStages).toBe(0)
    })

    test('한 쪽 크기를 넘는 판도 모두 본다', async () => {
        const runs = Array.from({ length: DAN_BACKFILL_PAGE + 1 }, (_, index) =>
            danRun(FIRST_DAN_COURSE, { id: `cs_${String(index).padStart(5, '0')}`, userId: `user-${index}`, playedAt: DAN_PLAYED_AT + index }),
        )
        const { service, counts } = createService({ runs })
        expect(await service.awardForCourses({ courseHashes: hashes, dryRun: false })).toEqual({
            scanned: DAN_BACKFILL_PAGE + 1,
            eligible: DAN_BACKFILL_PAGE + 1,
            awarded: DAN_BACKFILL_PAGE + 1,
        })
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
