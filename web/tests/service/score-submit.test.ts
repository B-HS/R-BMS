import { describe, expect, test } from 'bun:test'
import { judgeBreakdownSchema } from '@server/dto/common'
import { scoreSubmissionSchema, type ScoreSubmissionInput } from '@server/dto/score'
import { BUILD_TRUST } from '@server/service/domain/score/ranked-policy'
import {
    createScoreService,
    isJudgeCountConsistent,
    isPlayedAtPlausible,
    judgedNoteCount,
    type ScoreInsertRow,
    type ScoreServiceDb,
} from '@server/service/domain/score/score.service'

const CHART_SHA = 'f'.repeat(64)
const PLAYED_AT = 1700000000000

const submission = (overrides: Partial<Record<string, unknown>> = {}): ScoreSubmissionInput => {
    const { options: optionOverrides, ...rest } = overrides
    return scoreSubmissionSchema.parse({
        api_version: 1,
        chart: { md5: 'a'.repeat(32), sha256: CHART_SHA },
        player: { id: 'alice' },
        mode: 'BEAT_7K',
        clear: 'Hard',
        ex_score: 1488,
        max_ex_score: 1624,
        judge: { pgreat: 712, great: 64, good: 21, bad: 8, poor: 5, miss: 2, fast: 30, slow: 40, combobreak: 15 },
        max_combo: 540,
        total_notes: 812,
        minbp: 15,
        gauge_value: 86,
        options: {
            gauge: 'Hard',
            random: 'Random',
            random_p2: null,
            scratch_auto: false,
            lntype: 1,
            input_device: 'keyboard',
            assist: [],
            judge_rate: 100,
            ...(optionOverrides as Record<string, unknown> | undefined),
        },
        played_at: PLAYED_AT,
        client: 'rbms/0.1',
        ...rest,
    })
}

type Recorder = {
    inserted: ScoreInsertRow[]
    bests: { scoreId: string; clear: number; exScore: number }[]
    audits: number
    auditUserAgents: (string | null)[]
    idempotencyLookups: number
}

const createStub = (options: { existingScoreId?: string | null; currentBest?: { clear: number; exScore: number; minbp: number } | null } = {}) => {
    const recorder: Recorder = { inserted: [], bests: [], audits: 0, auditUserAgents: [], idempotencyLookups: 0 }
    const db: ScoreServiceDb = {
        findScoreIdByIdempotency: async () => {
            recorder.idempotencyLookups += 1
            return options.existingScoreId ?? null
        },
        insertScore: async (row) => {
            recorder.inserted.push(row)
        },
        getChartBest: async () => options.currentBest ?? null,
        upsertChartBest: async (row) => {
            recorder.bests.push({ scoreId: row.scoreId, clear: row.clear, exScore: row.exScore })
        },
        countBetterBests: async () => 2,
        getRanking: async () => [],
        getBestByLoginId: async () => null,
        getScoreById: async () => null,
        getRivalUserIds: async () => [],
        insertAudit: async (row) => {
            recorder.audits += 1
            recorder.auditUserAgents.push(row.userAgent)
        },
    }
    let sequence = 0
    const service = createScoreService({
        db,
        newId: (prefix) => `${prefix}${++sequence}`,
        now: () => PLAYED_AT,
    })
    return { recorder, service }
}

const alice = { id: 'usr_1', loginId: 'alice', name: 'Alice' }

describe('scoreService.submit', () => {
    test('정상 제출은 score 를 남기고 랭킹 순위를 계산한다', async () => {
        const { recorder, service } = createStub()
        const result = await service.submit({
            input: submission(),
            chartSha256: CHART_SHA,
            chartMd5: 'a'.repeat(32),
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: '127.0.0.1',
            userAgent: 'rbms',
        })
        expect(result.duplicated).toBe(false)
        expect(result.ranked).toBe(true)
        expect(result.response.accepted).toBe(true)
        expect(result.response.rank).toBe(3)
        expect(result.response.previous_best).toBeNull()
        expect(result.response.message).toBe('saved')
        expect(recorder.inserted).toHaveLength(1)
        expect(recorder.inserted[0].exScore).toBe(1488)
        expect(recorder.inserted[0].clear).toBe(6)
        expect(recorder.audits).toBe(1)
    })

    test('같은 (player, chart, played_at) 재전송은 기존 score_id 를 반환하고 새로 만들지 않는다', async () => {
        const { recorder, service } = createStub({ existingScoreId: 'sc_existing' })
        const result = await service.submit({
            input: submission(),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.duplicated).toBe(true)
        expect(result.scoreId).toBe('sc_existing')
        expect(recorder.inserted).toHaveLength(0)
        expect(recorder.bests).toHaveLength(0)
        expect(result.response.message).toBe('duplicate submission ignored')
    })

    test('기존 베스트보다 램프가 높으면 chart_best 를 갱신한다', async () => {
        const { recorder, service } = createStub({ currentBest: { clear: 5, exScore: 1600, minbp: 1 } })
        await service.submit({
            input: submission(),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(recorder.bests).toHaveLength(1)
        expect(recorder.bests[0].clear).toBe(6)
    })

    test('기존 베스트보다 램프가 낮으면 chart_best 를 갱신하지 않는다', async () => {
        const { recorder, service } = createStub({ currentBest: { clear: 8, exScore: 1000, minbp: 0 } })
        const result = await service.submit({
            input: submission(),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(recorder.bests).toHaveLength(0)
        expect(recorder.inserted).toHaveLength(1)
        expect(result.response.previous_best).toBe(1000)
    })

    test('autoplay 제출은 unranked 이고 베스트를 갱신하지 않으며 순위가 없다', async () => {
        const { recorder, service } = createStub()
        const result = await service.submit({
            input: submission({ options: { autoplay: true } }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.ranked).toBe(false)
        expect(result.flags).toEqual(['AUTOPLAY'])
        expect(result.response.rank).toBeNull()
        expect(result.response.message).toBe('recorded (unranked: autoplay)')
        expect(recorder.bests).toHaveLength(0)
        expect(recorder.inserted[0].ranked).toBe(false)
    })

    test('guest 제출은 user_id 없이 저장되고 항상 unranked 다', async () => {
        const { recorder, service } = createStub()
        const result = await service.submit({
            input: submission({ player: { id: 'guest' } }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: null,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.ranked).toBe(false)
        expect(result.flags).toEqual(['GUEST'])
        expect(recorder.inserted[0].userId).toBeNull()
        expect(recorder.inserted[0].guestName).toBe('guest')
        expect(recorder.bests).toHaveLength(0)
    })

    test('guest 제출은 멱등 선조회를 하지 않고 항상 새 기록을 남긴다', async () => {
        const { recorder, service } = createStub({ existingScoreId: 'sc_other_guest' })
        const result = await service.submit({
            input: submission({ player: { id: 'guest' } }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: null,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(recorder.idempotencyLookups).toBe(0)
        expect(result.duplicated).toBe(false)
        expect(recorder.inserted).toHaveLength(1)
    })

    test('user_agent 는 255자로 잘라 감사 로그에 남긴다', async () => {
        const { recorder, service } = createStub()
        await service.submit({
            input: submission(),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: 'u'.repeat(400),
        })
        expect(recorder.auditUserAgents[0]).toHaveLength(255)
    })

    test('미등록 빌드도 기본 설정에서는 ranked 로 chart_best 를 갱신한다', async () => {
        const { recorder, service } = createStub()
        const result = await service.submit({
            input: submission(),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.UNKNOWN,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.ranked).toBe(true)
        expect(result.flags).toEqual(['UNKNOWN_BUILD'])
        expect(result.response.rank).toBe(3)
        expect(recorder.bests).toHaveLength(1)
    })

    test('ex_score 가 0 이면 judge 에서 EX 를 유도한다', async () => {
        const { recorder, service } = createStub()
        await service.submit({
            input: submission({ ex_score: 0, minbp: 0 }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(recorder.inserted[0].exScore).toBe(1488)
        expect(recorder.inserted[0].minbp).toBe(15)
    })
})

describe('isPlayedAtPlausible', () => {
    test('오프라인 후 동기화를 위해 과거 시각은 제한 없이 통과한다', () => {
        expect(isPlayedAtPlausible(PLAYED_AT, PLAYED_AT)).toBe(true)
        expect(isPlayedAtPlausible(PLAYED_AT - 400 * 24 * 60 * 60 * 1000, PLAYED_AT)).toBe(true)
    })

    test('서버 시각보다 5분 넘게 미래면 거부한다', () => {
        expect(isPlayedAtPlausible(PLAYED_AT + 4 * 60 * 1000, PLAYED_AT)).toBe(true)
        expect(isPlayedAtPlausible(PLAYED_AT + 6 * 60 * 1000, PLAYED_AT)).toBe(false)
    })
})

describe('judge 합계 일관성', () => {
    const judge = judgeBreakdownSchema.parse({
        pgreat: 700,
        great: 60,
        good: 20,
        bad: 8,
        poor: 12,
        miss: 2,
        fast: 0,
        slow: 0,
        combobreak: 0,
        empty_poor: 4,
    })

    test('empty_poor 를 제외한 판정 합을 센다', () => {
        expect(judgedNoteCount(judge)).toBe(798)
    })

    test('판정 합이 total_notes 를 넘으면 거부한다', () => {
        expect(isJudgeCountConsistent(judge, 812)).toBe(true)
        expect(isJudgeCountConsistent(judge, 798)).toBe(true)
        expect(isJudgeCountConsistent(judge, 500)).toBe(false)
    })

    test('total_notes 가 0 인 구버전 제출은 검사를 건너뛴다', () => {
        expect(isJudgeCountConsistent(judge, 0)).toBe(true)
    })
})
