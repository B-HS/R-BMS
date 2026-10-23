import { describe, expect, test } from 'bun:test'
import { clearLampToId, judgeBreakdownSchema } from '@server/dto/common'
import { scoreSubmissionSchema, submitResponseSchema, type ScoreSubmissionInput } from '@server/dto/score'
import { BUILD_TRUST } from '@server/service/domain/score/ranked-policy'
import { isBetterBest } from '@server/service/domain/score/best-policy'
import {
    createScoreService,
    DUPLICATE_SUBMISSION_MESSAGE,
    findSubmissionIntegrityFailure,
    isClearLampConsistent,
    isExScoreConsistent,
    isJudgeCountConsistent,
    isPlayedAtPlausible,
    judgedNoteCount,
    type ScoreChartRankedRow,
    type ScoreInsertRow,
    type ScoreServiceDb,
    type StoredRunForBest,
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

const storedRun = (overrides: Partial<StoredRunForBest> & { id: string }): StoredRunForBest => ({
    clear: clearLampToId('Hard'),
    assistGrade: 0,
    exScore: 1488,
    minbp: 15,
    maxCombo: 540,
    ranked: true,
    scratchAuto: false,
    ...overrides,
})

type Recorder = {
    inserted: ScoreInsertRow[]
    calls: string[]
    bests: { scoreId: string; clear: number; exScore: number; scratchAuto: boolean }[]
    bestLookups: boolean[]
    rankScopes: boolean[]
    audits: number
    auditUserAgents: (string | null)[]
    idempotencyLookups: number
    idempotencyWindows: { playedAtFrom: number; playedAtTo: number }[]
}

const createStub = (
    options: {
        existingRun?: StoredRunForBest | null
        currentBest?: { clear: number; assistGrade: 0 | 1 | 2 | 3 | 4; exScore: number; minbp: number } | null
        winsBest?: boolean
        rankedRows?: ScoreChartRankedRow[]
        remembersInserted?: boolean
        bestFailures?: number
    } = {},
) => {
    const recorder: Recorder = {
        inserted: [],
        calls: [],
        bests: [],
        bestLookups: [],
        rankScopes: [],
        audits: 0,
        auditUserAgents: [],
        idempotencyLookups: 0,
        idempotencyWindows: [],
    }
    const db: ScoreServiceDb = {
        findScoreByIdempotency: async ({ playedAtFrom, playedAtTo }) => {
            recorder.idempotencyLookups += 1
            recorder.idempotencyWindows.push({ playedAtFrom, playedAtTo })
            const [inserted] = recorder.inserted
            if (options.remembersInserted && inserted) return { ...inserted, assistGrade: 0 }
            return options.existingRun ?? null
        },
        insertScore: async (row) => {
            recorder.inserted.push(row)
        },
        getChartBest: async ({ scratchAuto }) => {
            recorder.bestLookups.push(scratchAuto)
            return options.currentBest ?? null
        },
        upsertChartBestIfBetter: async (row) => {
            recorder.calls.push('best')
            if (recorder.calls.filter((call) => call === 'best').length <= (options.bestFailures ?? 0)) throw LOCK_DEADLOCK
            recorder.bests.push({ scoreId: row.scoreId, clear: row.clear, exScore: row.exScore, scratchAuto: row.scratchAuto })
            return options.winsBest ?? true
        },
        countBetterBests: async ({ includeScratchAuto }) => {
            recorder.calls.push('rank')
            recorder.rankScopes.push(includeScratchAuto)
            return 2
        },
        getRanking: async () => options.rankedRows ?? [],
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
        listDisplayedDans: async () => new Map(),
        newId: (prefix) => `${prefix}${++sequence}`,
        now: () => PLAYED_AT,
    })
    return { recorder, service }
}

const alice = { id: 'usr_1', loginId: 'alice', name: 'Alice' }

const LOCK_DEADLOCK = Object.assign(new Error('Deadlock found when trying to get lock'), { code: 'ER_LOCK_DEADLOCK' })

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
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual([])
        expect(result.response.is_new_best).toBe(true)
        expect(result.response.score_id).toBe(result.scoreId)
        expect(recorder.inserted).toHaveLength(1)
        expect(recorder.inserted[0].exScore).toBe(1488)
        expect(recorder.inserted[0].clear).toBe(6)
        expect(recorder.audits).toBe(1)
    })

    test('SCRATCH AUTO 판은 ranked 로 저장되고 SCRATCH AUTO 베스트와 포함 목록 순위를 쓴다', async () => {
        const { recorder, service } = createStub()
        const result = await service.submit({
            input: submission({ options: { scratch_auto: true, assist: ['AUTO_SCRATCH'] } }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.ranked).toBe(true)
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual(['SCRATCH_AUTO'])
        expect(result.response.message).toBe('saved (scratch auto)')
        expect(result.response.rank).toBe(3)
        expect(result.response.is_new_best).toBe(true)
        expect(recorder.inserted[0].ranked).toBe(true)
        expect(recorder.inserted[0].scratchAuto).toBe(true)
        expect(recorder.bestLookups).toEqual([true])
        expect(recorder.bests).toEqual([{ scoreId: result.scoreId, clear: 6, exScore: 1488, scratchAuto: true }])
        expect(recorder.rankScopes).toEqual([true])
    })

    test('일반 판은 일반 베스트와 기본 목록 순위를 쓴다', async () => {
        const { recorder, service } = createStub()
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
        expect(recorder.inserted[0].scratchAuto).toBe(false)
        expect(recorder.bestLookups).toEqual([false])
        expect(recorder.bests[0].scratchAuto).toBe(false)
        expect(recorder.rankScopes).toEqual([false])
    })

    test('넓힌 판정 판은 SCRATCH AUTO 와 함께여도 unranked 이고 베스트·순위가 없다', async () => {
        const { recorder, service } = createStub()
        const result = await service.submit({
            input: submission({ options: { scratch_auto: true, assist: ['AUTO_SCRATCH', 'CUSTOM_JUDGE'], judge_rate: 120 } }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.response.ranked).toBe(false)
        expect(result.response.rank).toBeNull()
        expect(result.response.message).toBe('recorded (unranked: assist,judge_width)')
        expect(recorder.bests).toHaveLength(0)
        expect(recorder.rankScopes).toEqual([])
    })

    test('SCRATCH AUTO 판의 재전송은 포함 목록 순위를 돌려준다', async () => {
        const { recorder, service } = createStub({ existingRun: storedRun({ id: 'sc_existing', scratchAuto: true }) })
        const result = await service.submit({
            input: submission({ options: { scratch_auto: true, assist: ['AUTO_SCRATCH'] } }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(result.response.rank).toBe(3)
        expect(result.response.flags).toEqual(['SCRATCH_AUTO'])
        expect(recorder.rankScopes).toEqual([true])
    })

    test('같은 (player, chart, played_at) 재전송은 기존 score_id 를 반환하고 새로 만들지 않으며, 저장된 판의 값으로 best 를 한 번 맞춘 뒤 순위를 센다', async () => {
        const { recorder, service } = createStub({ existingRun: storedRun({ id: 'sc_existing', exScore: 1400 }) })
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
        expect(recorder.bests).toEqual([{ scoreId: 'sc_existing', clear: clearLampToId('Hard'), exScore: 1400, scratchAuto: false }])
        expect(recorder.calls).toEqual(['best', 'rank'])
        expect(result.response.message).toBe('duplicate submission ignored')
        expect(result.response.score_id).toBe('sc_existing')
        expect(result.response.is_new_best).toBe(false)
        expect(result.response.rank).toBe(3)
    })

    test('저장된 판이 unranked 면 재전송은 best 를 건드리지 않고 순위도 돌려주지 않는다', async () => {
        const { recorder, service } = createStub({ existingRun: storedRun({ id: 'sc_unranked', ranked: false }) })
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
        expect(recorder.bests).toHaveLength(0)
        expect(result.response.rank).toBeNull()
    })

    test('점수는 남고 best 갱신이 실패한 판을 다시 보내면 best 가 그 판으로 바뀐다', async () => {
        const { recorder, service } = createStub({ remembersInserted: true, bestFailures: 1 })
        const send = () =>
            service.submit({
                input: submission(),
                chartSha256: CHART_SHA,
                chartMd5: null,
                user: alice,
                buildTrust: BUILD_TRUST.TRUSTED,
                requireTrustedBuild: false,
                ip: null,
                userAgent: null,
            })
        await expect(send()).rejects.toBe(LOCK_DEADLOCK)
        expect(recorder.inserted).toHaveLength(1)
        const resent = await send()
        expect(resent.duplicated).toBe(true)
        expect(resent.scoreId).toBe(recorder.inserted[0].id)
        expect(recorder.bests).toEqual([{ scoreId: recorder.inserted[0].id, clear: clearLampToId('Hard'), exScore: 1488, scratchAuto: false }])
        expect(recorder.calls.at(-1)).toBe('rank')
    })

    test('같은 초 안의 played_at 재전송은 기존 score_id 를 돌려준다', async () => {
        const { recorder, service } = createStub({ existingRun: storedRun({ id: 'sc_old' }) })
        const result = await service.submit({
            input: submission({ played_at: PLAYED_AT + 456 }),
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })
        expect(recorder.idempotencyWindows).toEqual([{ playedAtFrom: 1700000000000, playedAtTo: 1700000000999 }])
        expect(result.duplicated).toBe(true)
        expect(result.response.score_id).toBe('sc_old')
        expect(result.response.message).toBe(DUPLICATE_SUBMISSION_MESSAGE)
        expect(recorder.inserted).toHaveLength(0)
    })

    test('중복 응답 문구는 클라이언트 계약 문자열 그대로다', () => {
        expect(DUPLICATE_SUBMISSION_MESSAGE).toBe('duplicate submission ignored')
    })

    test('기존 베스트보다 램프가 높으면 chart_best 를 갱신한다', async () => {
        const { recorder, service } = createStub({ currentBest: { clear: 5, assistGrade: 0, exScore: 1600, minbp: 1 } })
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
        const { recorder, service } = createStub({ currentBest: { clear: 8, assistGrade: 0, exScore: 1000, minbp: 0 } })
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
        expect(result.response.is_new_best).toBe(false)
        expect(result.response.ranked).toBe(true)
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
        expect(result.response.ranked).toBe(false)
        expect(result.response.flags).toEqual(['AUTOPLAY'])
        expect(result.response.is_new_best).toBe(false)
        expect(result.response.score_id).toBe(result.scoreId)
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
        expect(result.response.ranked).toBe(false)
        expect(result.response.flags).toEqual(['GUEST'])
        expect(result.response.score_id).toBe(result.scoreId)
        expect(recorder.inserted[0].userId).toBeNull()
        expect(recorder.inserted[0].guestName).toBe('guest')
        expect(recorder.bests).toHaveLength(0)
    })

    test('guest 제출은 멱등 선조회를 하지 않고 항상 새 기록을 남긴다', async () => {
        const { recorder, service } = createStub({ existingRun: storedRun({ id: 'sc_other_guest' }) })
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
        expect(result.response.ranked).toBe(true)
        expect(result.response.flags).toEqual(['UNKNOWN_BUILD'])
        expect(result.response.is_new_best).toBe(true)
        expect(recorder.bests).toHaveLength(1)
    })

    test('응답 본문은 SubmitResponse 슈퍼셋 스키마를 그대로 만족한다', async () => {
        const { service } = createStub()
        const cases = await Promise.all(
            [submission(), submission({ options: { assist: ['CUSTOM_JUDGE'] } })].map((input) =>
                service.submit({
                    input,
                    chartSha256: CHART_SHA,
                    chartMd5: null,
                    user: alice,
                    buildTrust: BUILD_TRUST.TRUSTED,
                    requireTrustedBuild: false,
                    ip: null,
                    userAgent: null,
                }),
            ),
        )
        for (const result of cases) {
            const parsed = submitResponseSchema.parse(result.response)
            expect(Object.keys(parsed).sort()).toEqual(['accepted', 'flags', 'is_new_best', 'message', 'previous_best', 'rank', 'ranked', 'score_id'])
        }
        expect(cases[1].response.flags).toEqual(['ASSIST'])
        expect(cases[1].response.ranked).toBe(false)
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

describe('최고 기록 경합 (IR-1)', () => {
    const ranked = (overrides: Partial<ScoreChartRankedRow>): ScoreChartRankedRow => ({
        scoreId: 'sc_1',
        userId: 'usr_1',
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

    const submitAs = (service: ReturnType<typeof createStub>['service'], input: ScoreSubmissionInput) =>
        service.submit({
            input,
            chartSha256: CHART_SHA,
            chartMd5: null,
            user: alice,
            buildTrust: BUILD_TRUST.TRUSTED,
            requireTrustedBuild: false,
            ip: null,
            userAgent: null,
        })

    test('DB 가 더 좋은 최고 기록이 이미 있다고 답하면 is_new_best 는 false 다', async () => {
        const { service } = createStub({ winsBest: false })
        const result = await submitAs(service, submission())
        expect(result.response.is_new_best).toBe(false)
        expect(result.response.ranked).toBe(true)
    })

    test('같은 차트에 동시에 들어온 두 판 중 높은 판만 최고 기록으로 남는다', async () => {
        const stored: { exScore: number; clear: number; assistGrade: number; minbp: number; scoreId: string }[] = []
        const sequence = { value: 0 }
        let readsDone = 0
        let release: () => void = () => undefined
        const bothRead = new Promise<void>((resolve) => {
            release = resolve
        })
        const db: ScoreServiceDb = {
            findScoreByIdempotency: async () => null,
            insertScore: async () => undefined,
            getChartBest: async () => {
                readsDone += 1
                if (readsDone === 2) release()
                await bothRead
                return null
            },
            upsertChartBestIfBetter: async (row) => {
                const [current] = stored
                if (current && !isBetterBest(row, current)) return false
                stored.splice(0, stored.length, {
                    exScore: row.exScore,
                    clear: row.clear,
                    assistGrade: row.assistGrade,
                    minbp: row.minbp,
                    scoreId: row.scoreId,
                })
                return true
            },
            countBetterBests: async () => 0,
            getRanking: async () => [],
            getBestByLoginId: async () => null,
            getScoreById: async () => null,
            getRivalUserIds: async () => [],
            insertAudit: async () => undefined,
        }
        const service = createScoreService({
            db,
            listDisplayedDans: async () => new Map(),
            newId: (prefix) => `${prefix}${++sequence.value}`,
            now: () => PLAYED_AT,
        })
        const high = submission({ ex_score: 1488 })
        const low = submission({
            ex_score: 1000,
            judge: { pgreat: 400, great: 200, good: 21, bad: 8, poor: 5, miss: 2, fast: 0, slow: 0, combobreak: 0 },
            played_at: PLAYED_AT + 5000,
        })
        const [highResult, lowResult] = await Promise.all([submitAs(service, high), submitAs(service, low)])
        expect(stored).toHaveLength(1)
        expect(stored[0].exScore).toBe(1488)
        expect(highResult.response.is_new_best).toBe(true)
        expect(lowResult.response.is_new_best).toBe(false)
    })

    test('getRanking 은 DB 가 센 공동 순위를 그대로 응답 순위로 쓴다', async () => {
        const { service } = createStub({
            rankedRows: [
                ranked({ scoreId: 'sc_a', loginId: 'a', rank: 1 }),
                ranked({ scoreId: 'sc_b', loginId: 'b', rank: 1 }),
                ranked({ scoreId: 'sc_c', loginId: 'c', rank: 3 }),
            ],
        })
        const rows = await service.getRanking({ chartSha256: CHART_SHA, limit: 50, page: 2, includeScratchAuto: false })
        expect(rows.map((row) => row.rank)).toEqual([1, 1, 3])
    })
})

describe('EX 점수와 노트 수 대조 (IR-4)', () => {
    const judge = judgeBreakdownSchema.parse({
        pgreat: 1,
        great: 0,
        good: 0,
        bad: 0,
        poor: 0,
        miss: 0,
        fast: 0,
        slow: 0,
        combobreak: 0,
    })

    test('EX 는 PGREAT 2점과 GREAT 1점의 합과 같아야 하고 0 은 서버가 유도한다', () => {
        expect(isExScoreConsistent(judge, 2)).toBe(true)
        expect(isExScoreConsistent(judge, 0)).toBe(true)
        expect(isExScoreConsistent(judge, 999999)).toBe(false)
        expect(isExScoreConsistent(judge, 3)).toBe(false)
    })

    test('early/late 로 나눠 보낸 PGREAT 도 같은 합으로 센다', () => {
        const split = judgeBreakdownSchema.parse({ ...judge, pgreat: 4, epg: 3, lpg: 1, egr: 2, lgr: 1, great: 3 })
        expect(isExScoreConsistent(split, 11)).toBe(true)
        expect(isExScoreConsistent(split, 12)).toBe(false)
    })

    const body = ({ judge: judged = {}, ...rest }: { judge?: Record<string, number> } & Record<string, unknown>) =>
        submission({ clear: 'Normal', ex_score: 0, max_ex_score: 0, minbp: 0, ...rest, judge: { ...judge, ...judged } })

    test('EX 가 판정과 어긋난 본문은 ex_score 필드로 거절한다', () => {
        expect(findSubmissionIntegrityFailure(body({ total_notes: 812, ex_score: 999999 }))).toEqual({
            field: 'ex_score',
            reason: 'ex_score does not match the judge counts',
        })
        expect(findSubmissionIntegrityFailure(body({ total_notes: 812, ex_score: 2 }))).toBeNull()
    })

    test('분리 필드(epg)만 키우고 합계(pgreat)를 0 으로 둔 위조는 노트 수에 묶여 거절된다 (R2-ACC-1)', () => {
        const forged = body({ judge: { pgreat: 0, epg: 100_000 }, total_notes: 500, ex_score: 200_000, clear: 'Max' })
        expect(findSubmissionIntegrityFailure(forged)).toEqual({ field: 'judge', reason: 'judge count exceeds total_notes' })
        expect(judgedNoteCount(forged.judge, 'BEAT_7K')).toBe(100_000)
    })

    test('total_notes 를 0 으로 보내 상한을 피할 수 없다', () => {
        expect(findSubmissionIntegrityFailure(body({ judge: { pgreat: 600 }, total_notes: 0, ex_score: 1200 }))).toEqual({
            field: 'judge',
            reason: 'judge count exceeds total_notes',
        })
        expect(findSubmissionIntegrityFailure(body({ judge: { pgreat: 0, miss: 3 }, total_notes: 0, clear: 'NoPlay' }))).toBeNull()
    })

    test('max_ex_score 는 0 이거나 노트당 2 이하다', () => {
        expect(findSubmissionIntegrityFailure(body({ total_notes: 500, max_ex_score: 1000 }))).toBeNull()
        expect(findSubmissionIntegrityFailure(body({ total_notes: 500, max_ex_score: 1001 }))).toEqual({
            field: 'max_ex_score',
            reason: 'max_ex_score exceeds two per note',
        })
    })

    test('minbp 는 0 이거나 BAD+POOR+空POOR 이상이다 (게이지가 끝난 판은 못 친 노트가 더해진다)', () => {
        const judged = { pgreat: 400, poor: 100 }
        expect(findSubmissionIntegrityFailure(body({ judge: judged, total_notes: 500, minbp: 1 }))).toEqual({
            field: 'minbp',
            reason: 'minbp is below the BAD, POOR and empty POOR count',
        })
        expect(findSubmissionIntegrityFailure(body({ judge: judged, total_notes: 500, minbp: 100 }))).toBeNull()
        expect(findSubmissionIntegrityFailure(body({ judge: judged, total_notes: 600, minbp: 200, clear: 'Failed' }))).toBeNull()
        expect(findSubmissionIntegrityFailure(body({ judge: judged, total_notes: 500, minbp: 0 }))).toBeNull()
    })

    test('판정과 무관한 MAX 램프는 clear 필드로 거절한다 (검증자 missed 2)', () => {
        expect(findSubmissionIntegrityFailure(body({ judge: { pgreat: 400, poor: 100 }, total_notes: 500, clear: 'Max' }))).toEqual({
            field: 'clear',
            reason: 'clear lamp is not reachable with these judge counts',
        })
    })
})

describe('램프와 판정의 일관성 (원본 BMSPlayer.java:868 의 풀콤 조건)', () => {
    const counts = (overrides: Record<string, number>) =>
        judgeBreakdownSchema.parse({ pgreat: 0, great: 0, good: 0, bad: 0, poor: 0, miss: 0, fast: 0, slow: 0, combobreak: 0, ...overrides })

    test('MAX 는 GREAT·GOOD 이 없고, PERFECT 는 GOOD 이 없다', () => {
        expect(isClearLampConsistent('Max', counts({ pgreat: 10 }), 'BEAT_7K')).toBe(true)
        expect(isClearLampConsistent('Max', counts({ pgreat: 9, great: 1 }), 'BEAT_7K')).toBe(false)
        expect(isClearLampConsistent('Perfect', counts({ pgreat: 9, great: 1 }), 'BEAT_7K')).toBe(true)
        expect(isClearLampConsistent('Perfect', counts({ pgreat: 9, good: 1 }), 'BEAT_7K')).toBe(false)
        expect(isClearLampConsistent('FullCombo', counts({ pgreat: 8, great: 1, good: 1 }), 'BEAT_7K')).toBe(true)
    })

    test('풀콤 램프는 POOR 가 하나라도 있으면 불가능하고, BAD 는 노트를 가져가는 모드에서만 불가능하다', () => {
        for (const lamp of ['FullCombo', 'Perfect', 'Max'] as const) {
            expect(isClearLampConsistent(lamp, counts({ pgreat: 9, poor: 1 }), 'BEAT_7K')).toBe(false)
            expect(isClearLampConsistent(lamp, counts({ pgreat: 9, poor: 1 }), 'POPN_9K')).toBe(false)
            expect(isClearLampConsistent(lamp, counts({ pgreat: 9, bad: 1 }), 'BEAT_7K')).toBe(false)
            expect(isClearLampConsistent(lamp, counts({ pgreat: 9, bad: 1 }), 'BEAT_5K')).toBe(false)
            expect(isClearLampConsistent(lamp, counts({ pgreat: 9, bad: 1 }), 'POPN_9K')).toBe(true)
        }
    })

    test("空POOR(miss) 는 노트를 가져가지 않아 어느 풀콤 램프도 막지 않는다 (7K 는 콤보를 잇고, 5K·pop'n 은 첫 노트 앞이면 된다)", () => {
        for (const mode of ['BEAT_7K', 'BEAT_5K', 'POPN_9K', 'KEYBOARD_24K']) {
            expect(isClearLampConsistent('Max', counts({ pgreat: 10, miss: 3, empty_poor: 3 }), mode)).toBe(true)
        }
    })

    test('게이지 램프와 FAILED·어시스트 램프는 판정과 무관하다', () => {
        for (const lamp of ['NoPlay', 'Failed', 'AssistEasy', 'LightAssistEasy', 'Easy', 'Normal', 'Hard', 'ExHard'] as const) {
            expect(isClearLampConsistent(lamp, counts({ pgreat: 1, bad: 5, poor: 5 }), 'BEAT_7K')).toBe(true)
        }
    })

    test('분리 필드만 보낸 판정도 같은 수로 읽는다', () => {
        expect(isClearLampConsistent('Max', counts({ pgreat: 10, epr: 1 }), 'BEAT_7K')).toBe(false)
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
        miss: 4,
        fast: 0,
        slow: 0,
        combobreak: 0,
        empty_poor: 4,
    })

    test('노트를 소비한 판정(PG·GR·GD·BD·POOR)만 센다', () => {
        expect(judgedNoteCount(judge, 'BEAT_7K')).toBe(800)
    })

    test('판정 합이 total_notes 를 넘으면 거부한다', () => {
        expect(isJudgeCountConsistent(judge, 812, 'BEAT_7K')).toBe(true)
        expect(isJudgeCountConsistent(judge, 800, 'BEAT_7K')).toBe(true)
        expect(isJudgeCountConsistent(judge, 799, 'BEAT_7K')).toBe(false)
    })

    test('空POOR 가 見逃し POOR 보다 많은 완주 판을 받는다(사용자 판 #52)', () => {
        const userRun = judgeBreakdownSchema.parse({
            pgreat: 1500,
            great: 250,
            good: 50,
            bad: 15,
            poor: 20,
            miss: 43,
            empty_poor: 43,
            fast: 0,
            slow: 0,
            combobreak: 0,
        })
        expect(judgedNoteCount(userRun, 'BEAT_7K')).toBe(1835)
        expect(isJudgeCountConsistent(userRun, 1835, 'BEAT_7K')).toBe(true)
    })

    test('POPN_9K 는 노트를 소비하지 않는 BAD 를 세지 않는다', () => {
        const popn = judgeBreakdownSchema.parse({
            pgreat: 90,
            great: 5,
            good: 0,
            bad: 8,
            poor: 5,
            miss: 0,
            empty_poor: 0,
            fast: 0,
            slow: 0,
            combobreak: 0,
        })
        expect(judgedNoteCount(popn, 'POPN_9K')).toBe(100)
        expect(isJudgeCountConsistent(popn, 100, 'POPN_9K')).toBe(true)
        expect(isJudgeCountConsistent(popn, 100, 'popn-9k')).toBe(true)
        expect(isJudgeCountConsistent(popn, 100, 'BEAT_7K')).toBe(false)
        expect(judgedNoteCount(popn, 'BEAT_7K')).toBe(108)
    })

    test('total_notes 가 0 이면 노트를 가져간 판정도 0 이어야 한다', () => {
        expect(isJudgeCountConsistent(judge, 0, 'BEAT_7K')).toBe(false)
    })
})
