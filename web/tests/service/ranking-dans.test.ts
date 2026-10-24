import { describe, expect, test } from 'bun:test'
import { z } from 'zod'
import { clearLampToId } from '@server/dto/common'
import { displayedDanSchema, type DisplayedDan } from '@server/dto/dan'
import { createDanService } from '@server/service/domain/dan/dan.service'
import { SCORE_FLAG } from '@server/service/domain/score/ranked-policy'
import { createScoreService, toScoreRecord, type ScoreChartRankedRow, type ScoreServiceDb } from '@server/service/domain/score/score.service'
import { createMemoryDanDb } from '../support/dan-memory-db'
import { danRun, FIRST_DAN_COURSE, SECOND_DAN_COURSE, STELLA_DAN_COURSE } from '../support/dan-fixture'

const CHART_SHA = 'f'.repeat(64)
const PLAYED_AT = 1_700_000_000_000
const RANKING_LIMIT_MAX = 500
const RANKING_PAGE = 3

const ALICE = 'u_alice'
const BOB = 'u_bob'
const CAROL = 'u_carol'

const GENOSIDE_SECOND: DisplayedDan = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '二段', assisted: false }
const GENOSIDE_FIRST: DisplayedDan = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '初段', assisted: false }
const GENOSIDE_ASSISTED_SECOND: DisplayedDan = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: 'A-二段', assisted: true }
const STELLA_ZERO: DisplayedDan = { system: 'stella', name: 'Stella', label: 'st0', assisted: false }

const rankedRow = (overrides: Partial<ScoreChartRankedRow>): ScoreChartRankedRow => ({
    scoreId: 'sc_1',
    userId: ALICE,
    loginId: 'alice',
    playerName: 'Alice',
    clear: 6,
    assistGrade: 0,
    exScore: 1000,
    maxCombo: 100,
    minbp: 10,
    playedAt: PLAYED_AT,
    lntype: 0,
    option: 0,
    totalNotes: 812,
    scratchAuto: false,
    judge: null,
    rank: 1,
    ...overrides,
})

const withoutDans = (record: { extra: Record<string, unknown> }) => ({
    ...record,
    extra: Object.fromEntries(Object.entries(record.extra).filter(([key]) => key !== 'dans')),
})

type RankingQueryParams = Parameters<ScoreServiceDb['getRanking']>[0]

const createRankingStub = (options: {
    rows: ScoreChartRankedRow[]
    rivalUserIds?: string[]
    listDisplayedDans: (userIds: string[]) => Promise<Map<string, DisplayedDan[]>>
}) => {
    const rankingQueries: RankingQueryParams[] = []
    const rivalOwners: string[] = []
    const db: ScoreServiceDb = {
        findScoreByIdempotency: async () => null,
        insertScore: async () => undefined,
        getChartBest: async () => null,
        upsertChartBestIfBetter: async () => false,
        countBetterBests: async () => 0,
        getRanking: async (params) => {
            rankingQueries.push(params)
            return options.rows
        },
        getBestByLoginId: async () => options.rows[0] ?? null,
        getScoreById: async () => options.rows[0] ?? null,
        getRivalUserIds: async (loginId) => {
            rivalOwners.push(loginId)
            return options.rivalUserIds ?? []
        },
        insertAudit: async () => undefined,
    }
    const service = createScoreService({ db, listDisplayedDans: options.listDisplayedDans, newId: (prefix) => `${prefix}1`, now: () => PLAYED_AT })
    return { service, rankingQueries, rivalOwners }
}

const askedDansOf = () => {
    const asked: string[][] = []
    const listDisplayedDans = async (userIds: string[]) => {
        asked.push(userIds)
        return new Map<string, DisplayedDan[]>([
            [ALICE, [GENOSIDE_SECOND, STELLA_ZERO]],
            [BOB, []],
        ])
    }
    return { asked, listDisplayedDans }
}

const ranking = { chartSha256: CHART_SHA, limit: 50, page: 1, includeScratchAuto: false }

describe('차트 순위 응답 줄의 표시 단위', () => {
    test('줄마다 extra.dans 가 붙고 단위 조회는 순위 줄의 사용자 전체로 한 번이다', async () => {
        const { asked, listDisplayedDans } = askedDansOf()
        const { service } = createRankingStub({
            rows: [
                rankedRow({ userId: ALICE, loginId: 'alice', replayId: 'rp_1', rank: 1 }),
                rankedRow({ userId: BOB, loginId: 'bob', scratchAuto: true, rank: 2 }),
                rankedRow({ userId: CAROL, loginId: 'carol', rank: 3 }),
            ],
            listDisplayedDans,
        })

        const rows = await service.getRanking(ranking)

        expect(asked).toEqual([[ALICE, BOB, CAROL]])
        expect(rows.map((row) => row.extra)).toEqual([
            { replay_id: 'rp_1', dans: [GENOSIDE_SECOND, STELLA_ZERO] },
            { scratch_auto: true, dans: [] },
            { dans: [] },
        ])
    })

    test('단위 조회가 모르는 사용자와 단위가 없는 사용자는 빈 배열이다', async () => {
        const { service } = createRankingStub({
            rows: [rankedRow({ userId: BOB, loginId: 'bob' }), rankedRow({ userId: CAROL, loginId: 'carol', rank: 2 })],
            listDisplayedDans: askedDansOf().listDisplayedDans,
        })

        const rows = await service.getRanking(ranking)

        expect(rows.map((row) => row.extra.dans)).toEqual([[], []])
    })

    test('순위, 정렬, 공동 순위와 나머지 필드는 단위가 없던 때와 같다', async () => {
        const stored = [
            rankedRow({ userId: ALICE, loginId: 'alice', rank: 1 }),
            rankedRow({ userId: BOB, loginId: 'bob', rank: 1, replayId: 'rp_2' }),
            rankedRow({ userId: CAROL, loginId: 'carol', rank: 3, scratchAuto: true }),
        ]
        const { service } = createRankingStub({ rows: stored, listDisplayedDans: askedDansOf().listDisplayedDans })

        const rows = await service.getRanking(ranking)

        expect(rows.map((row) => [row.player.id, row.rank])).toEqual([
            ['alice', 1],
            ['bob', 1],
            ['carol', 3],
        ])
        expect(rows.map(withoutDans)).toEqual(stored.map((row) => toScoreRecord(row, row.rank)))
    })

    test('빈 순위는 단위를 묻지 않는다', async () => {
        const { asked, listDisplayedDans } = askedDansOf()
        const { service } = createRankingStub({ rows: [], listDisplayedDans })

        expect(await service.getRanking(ranking)).toEqual([])
        expect(asked).toEqual([])
    })

    test('rival_of 는 라이벌 사용자로만 순위를 읽고 그 줄의 사용자만 단위를 묻는다', async () => {
        const { asked, listDisplayedDans } = askedDansOf()
        const { service, rankingQueries, rivalOwners } = createRankingStub({
            rows: [rankedRow({ userId: BOB, loginId: 'bob' })],
            rivalUserIds: [ALICE, BOB],
            listDisplayedDans,
        })

        const rows = await service.getRanking({ ...ranking, rivalOf: 'alice' })

        expect(rivalOwners).toEqual(['alice'])
        expect(rankingQueries.map((query) => query.userIds)).toEqual([[ALICE, BOB]])
        expect(asked).toEqual([[BOB]])
        expect(rows.map((row) => row.extra.dans)).toEqual([[]])
    })

    test('라이벌이 없으면 순위도 단위도 읽지 않는다', async () => {
        const { asked, listDisplayedDans } = askedDansOf()
        const { service, rankingQueries } = createRankingStub({ rows: [rankedRow({})], rivalUserIds: [], listDisplayedDans })

        expect(await service.getRanking({ ...ranking, rivalOf: 'nobody' })).toEqual([])
        expect(rankingQueries).toEqual([])
        expect(asked).toEqual([])
    })

    test('lnmode 와 쪽은 순위 질의에 그대로 가고 limit 500 줄도 단위 조회는 한 번이다', async () => {
        const { asked, listDisplayedDans } = askedDansOf()
        const rows = Array.from({ length: RANKING_LIMIT_MAX }, (_, index) =>
            rankedRow({ userId: `u_${index}`, loginId: `p${index}`, scoreId: `sc_${index}`, rank: index + 1 }),
        )
        const { service, rankingQueries } = createRankingStub({ rows, listDisplayedDans })

        const found = await service.getRanking({ ...ranking, limit: RANKING_LIMIT_MAX, page: RANKING_PAGE, lnmode: 2 })

        expect(rankingQueries).toEqual([
            {
                chartSha256: CHART_SHA,
                limit: RANKING_LIMIT_MAX,
                offset: (RANKING_PAGE - 1) * RANKING_LIMIT_MAX,
                lnmode: 2,
                userIds: undefined,
                includeScratchAuto: false,
            },
        ])
        expect(asked).toHaveLength(1)
        expect(asked[0]).toEqual(rows.map((row) => row.userId))
        expect(found).toHaveLength(RANKING_LIMIT_MAX)
    })

    test('내 최고 기록과 기록 상세 응답에는 dans 가 붙지 않는다', async () => {
        const { asked, listDisplayedDans } = askedDansOf()
        const { service } = createRankingStub({ rows: [rankedRow({ replayId: 'rp_1' })], listDisplayedDans })

        const best = await service.getBest({ chartSha256: CHART_SHA, loginId: 'alice', includeScratchAuto: false })
        const detail = await service.getById('sc_1')

        expect(best?.extra).toEqual({ replay_id: 'rp_1' })
        expect(detail?.extra).toEqual({ replay_id: 'rp_1' })
        expect(asked).toEqual([])
    })
})

describe('프로필과 같은 단위 묶음 조회로 붙는 순위 줄의 단위', () => {
    const createService = async () => {
        const memory = createMemoryDanDb({ courses: [FIRST_DAN_COURSE, SECOND_DAN_COURSE, STELLA_DAN_COURSE] })
        const lookups = { userDans: [] as string[][], displays: [] as string[][] }
        const danService = createDanService({
            db: {
                ...memory.db,
                listUserDans: async (userIds) => {
                    lookups.userDans.push(userIds)
                    return memory.db.listUserDans(userIds)
                },
                listDisplays: async (userIds) => {
                    lookups.displays.push(userIds)
                    return memory.db.listDisplays(userIds)
                },
            },
        })
        await danService.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_1', userId: ALICE }))
        await danService.awardFromRun(danRun(SECOND_DAN_COURSE, { id: 'cs_2', userId: ALICE }))
        await danService.awardFromRun(danRun(STELLA_DAN_COURSE, { id: 'cs_3', userId: ALICE }))
        await danService.awardFromRun(danRun(FIRST_DAN_COURSE, { id: 'cs_4', userId: BOB }))
        const { service } = createRankingStub({
            rows: [
                rankedRow({ userId: ALICE, loginId: 'alice', rank: 1 }),
                rankedRow({ userId: BOB, loginId: 'bob', rank: 2 }),
                rankedRow({ userId: CAROL, loginId: 'carol', rank: 3 }),
            ],
            listDisplayedDans: danService.listDisplayed,
        })
        return { danService, service, lookups }
    }

    test('가장 높은 단위를 체계마다 하나씩 붙이고 단위 테이블 조회는 두 번뿐이다', async () => {
        const { service, lookups } = await createService()

        const rows = await service.getRanking(ranking)

        expect(rows.map((row) => row.extra.dans)).toEqual([[GENOSIDE_SECOND, STELLA_ZERO], [GENOSIDE_FIRST], []])
        expect(lookups).toEqual({ userDans: [[ALICE, BOB, CAROL]], displays: [[ALICE, BOB, CAROL]] })
        for (const row of rows) expect(z.array(displayedDanSchema).parse(row.extra.dans)).toEqual(row.extra.dans)
    })

    test('남에게 보이지 않기로 한 단위는 순위 줄에도 없다', async () => {
        const { danService, service } = await createService()
        await danService.setDisplay(ALICE, { system: 'genoside2018_sp', mode: 'hidden' })
        await danService.setDisplay(BOB, { system: 'genoside2018_sp', mode: 'hidden' })

        const rows = await service.getRanking(ranking)

        expect(rows.map((row) => row.extra.dans)).toEqual([[STELLA_ZERO], [], []])
    })

    test('정식보다 높은 A-단위는 정식 다음에 A- 가 붙은 label 로 붙는다', async () => {
        const { danService, service } = await createService()
        await danService.awardFromRun(
            danRun(SECOND_DAN_COURSE, {
                id: 'cs_5',
                userId: BOB,
                scratchAuto: true,
                flags: [SCORE_FLAG.SCRATCH_AUTO],
                clear: clearLampToId('LightAssistEasy'),
            }),
        )

        const rows = await service.getRanking(ranking)

        expect(rows.map((row) => row.extra.dans)).toEqual([[GENOSIDE_SECOND, STELLA_ZERO], [GENOSIDE_FIRST, GENOSIDE_ASSISTED_SECOND], []])
        for (const row of rows) expect(z.array(displayedDanSchema).parse(row.extra.dans)).toEqual(row.extra.dans)
    })

    test('assisted 가 없는 옛 원소는 정식 단위로 읽힌다', () => {
        expect(displayedDanSchema.parse({ system: 'stella', name: 'Stella', label: 'st0' })).toEqual(STELLA_ZERO)
    })
})
