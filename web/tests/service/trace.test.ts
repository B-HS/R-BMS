import { describe, expect, test } from 'bun:test'
import { gunzipSync } from 'node:zlib'
import { TRACE_NOTE_KIND, TRACE_NOTE_STATE, TRACE_QUOTA_LEVEL, TRACE_USER_QUOTA, type TraceUploadInput } from '@server/dto/trace'
import { ANALYSIS_VERSION, PLAY_ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { countTraceJudgements, findTraceMismatch } from '@server/service/domain/trace/trace-check'
import {
    PLAY_ANALYSIS_READ_RECOMPUTE_MAX,
    TRACE_ANALYSIS_STATE,
    TRACE_UPLOAD_OUTCOME,
    toTraceStorage,
    traceQuotaLevelOf,
    type TraceInsertRow,
} from '@server/service/domain/trace/trace.service'
import { OTHER_USER_ID, TRACE_USER_ID, createMemoryTraceDb, createMemoryTraceService, readContractTrace, scoreRowFor } from '../support/trace-fixture'

const NORMAL = readContractTrace('trace-normal.json')
const ABANDONED = readContractTrace('trace-abandoned.json')
const GAUGE_EMPTY_AT_MS = 3000

/** A trace of `marks` as `[state, err_us, kind]`, one note per 100 ms on lanes 0 to 6 in turn. */
const traceOf = (marks: readonly (readonly [number, number, number])[], overrides: Partial<TraceUploadInput> = {}) =>
    ({
        ...NORMAL,
        notes: {
            time_us: marks.map((_, index) => (index + 1) * 100_000),
            lane: marks.map((_, index) => index % 7),
            kind: marks.map((mark) => mark[2]),
            state: marks.map((mark) => mark[0]),
            err_us: marks.map((mark) => mark[1]),
        },
        empty_poors: { time_us: [], lane: [] },
        ...overrides,
    }) satisfies TraceUploadInput

const scoreOf = (trace: TraceUploadInput, overrides: Parameters<typeof scoreRowFor>[1] = {}) =>
    scoreRowFor(trace, { ...countTraceJudgements(trace), ...overrides })

describe('countTraceJudgements — trace 가 더하는 판정 합계', () => {
    test('err_us 0 은 빠름이고, 음수만 늦음이다', () => {
        const counts = countTraceJudgements(
            traceOf([
                [TRACE_NOTE_STATE.PGREAT, 0, TRACE_NOTE_KIND.NORMAL],
                [TRACE_NOTE_STATE.PGREAT, -1, TRACE_NOTE_KIND.NORMAL],
                [TRACE_NOTE_STATE.BAD, 1, TRACE_NOTE_KIND.NORMAL],
            ]),
        )
        expect(counts).toMatchObject({ pgreat: 2, epg: 1, lpg: 1, bad: 1, ebd: 1, lbd: 0, poor: 0 })
    })

    test('중단한 판(failed_at_ms 없음)은 도달하지 못한 판정 노트를 POOR 로 세고, 일반 LN 끝은 세지 않는다', () => {
        const trace = traceOf([
            [TRACE_NOTE_STATE.GREAT, 5, TRACE_NOTE_KIND.LONG_HEAD],
            [TRACE_NOTE_STATE.NONE, 0, TRACE_NOTE_KIND.LONG_END],
            [TRACE_NOTE_STATE.NONE, 0, TRACE_NOTE_KIND.NORMAL],
            [TRACE_NOTE_STATE.NONE, 0, TRACE_NOTE_KIND.CHARGE_END],
            [TRACE_NOTE_STATE.POOR, -210_001, TRACE_NOTE_KIND.NORMAL],
        ])
        expect(countTraceJudgements(trace)).toMatchObject({ great: 1, poor: 3 })
    })

    test('게이지가 빈 판(failed_at_ms 있음)은 도달하지 못한 노트를 세지 않는다', () => {
        const trace = traceOf(
            [
                [TRACE_NOTE_STATE.GREAT, 5, TRACE_NOTE_KIND.NORMAL],
                [TRACE_NOTE_STATE.POOR, -210_001, TRACE_NOTE_KIND.NORMAL],
                [TRACE_NOTE_STATE.NONE, 0, TRACE_NOTE_KIND.NORMAL],
                [TRACE_NOTE_STATE.NONE, 0, TRACE_NOTE_KIND.CHARGE_END],
            ],
            { failed_at_ms: GAUGE_EMPTY_AT_MS },
        )
        expect(countTraceJudgements(trace)).toMatchObject({ great: 1, poor: 1 })
    })

    test('空POOR 는 empty_poors 의 개수다', () => {
        expect(countTraceJudgements(traceOf([], { empty_poors: { time_us: [1, 2, 3], lane: [0, 0, 7] } })).emptyPoor).toBe(3)
    })
})

describe('findTraceMismatch — 점수 행과의 일관성(허용 오차 없음)', () => {
    test('계약 파일의 Esc 포기 판은 남은 노트를 POOR 로 센 점수와 같고, 같은 판을 게이지 사망으로 읽으면 POOR 에서 어긋난다', () => {
        const score = scoreRowFor(ABANDONED, { pgreat: 4, great: 2, good: 1, poor: 25, epg: 3, lpg: 1, egr: 1, lgr: 1, lgd: 1 })
        expect(findTraceMismatch({ trace: ABANDONED, pathHash: ABANDONED.chart.md5, score })).toBeNull()
        expect(findTraceMismatch({ trace: { ...ABANDONED, failed_at_ms: GAUGE_EMPTY_AT_MS }, pathHash: ABANDONED.chart.md5, score })).toEqual({
            field: 'judge.poor',
            reason: 'score 25, trace 0',
        })
    })

    test('판정 칸이 하나라도 1 다르면 그 칸 이름으로 어긋난다', () => {
        const fields = ['pgreat', 'great', 'good', 'bad', 'poor', 'epg', 'lpg', 'egr', 'lgr', 'egd', 'lgd', 'ebd', 'lbd', 'emptyPoor'] as const
        const mismatched = fields.map((field) => {
            const score = scoreOf(NORMAL)
            return findTraceMismatch({ trace: NORMAL, pathHash: NORMAL.chart.md5, score: { ...score, [field]: score[field] + 1 } })?.field
        })
        expect(mismatched).toEqual([
            'judge.pgreat',
            'judge.great',
            'judge.good',
            'judge.bad',
            'judge.poor',
            'judge.epg',
            'judge.lpg',
            'judge.egr',
            'judge.lgr',
            'judge.egd',
            'judge.lgd',
            'judge.ebd',
            'judge.lbd',
            'judge.empty_poor',
        ])
    })

    test('9키는 BAD·POOR 와 그 빠름/늦음을 비교하지 않지만 나머지는 비교한다', () => {
        const trace = { ...NORMAL, mode: 'POPN_9K' }
        const score = scoreOf(trace, { mode: 'POPN_9K' })
        const distorted = { ...score, bad: score.bad + 2, ebd: score.ebd + 2, poor: score.poor + 1, lbd: score.lbd + 1 }
        expect(findTraceMismatch({ trace, pathHash: trace.chart.md5, score: distorted })).toBeNull()
        expect(findTraceMismatch({ trace, pathHash: trace.chart.md5, score: { ...distorted, great: score.great + 1 } })?.field).toBe('judge.great')
        expect(findTraceMismatch({ trace, pathHash: trace.chart.md5, score: { ...score, emptyPoor: 1 } })?.field).toBe('judge.empty_poor')
    })

    test('모드·LN 종류·RANDOM·SCRATCH AUTO 가 점수 행과 다르면 그 칸 이름으로 어긋난다', () => {
        const score = scoreOf(NORMAL)
        const fieldOf = (trace: TraceUploadInput, row: typeof score) => findTraceMismatch({ trace, pathHash: trace.chart.md5, score: row })?.field
        expect(fieldOf(NORMAL, score)).toBeUndefined()
        expect(fieldOf(NORMAL, { ...score, mode: 'BEAT_14K' })).toBe('mode')
        expect(fieldOf(NORMAL, { ...score, lntype: NORMAL.lntype + 1 })).toBe('lntype')
        expect(fieldOf(NORMAL, { ...score, random: 'Mirror' })).toBe('random')
        expect(fieldOf(NORMAL, { ...score, scratchAuto: true })).toBe('auto_lanes')
        const autoTrace = { ...NORMAL, auto_lanes: [false, false, false, false, false, false, false, true] }
        expect(fieldOf(autoTrace, score)).toBe('auto_lanes')
        expect(fieldOf(autoTrace, { ...score, scratchAuto: true })).toBeUndefined()
    })

    test('9키 예외는 점수 행의 모드를 따르고, trace 의 모드가 점수와 다르면 판정 칸보다 먼저 mode 로 어긋난다', () => {
        const trace = { ...NORMAL, mode: 'BEAT_7K' }
        const score = scoreOf(trace, { mode: 'POPN_9K' })
        expect(findTraceMismatch({ trace, pathHash: trace.chart.md5, score: { ...score, bad: score.bad + 1 } })?.field).toBe('mode')
    })

    test('본문의 차트와 경로의 해시가 점수의 차트여야 한다. 경로는 sha256 이어도 되고, 점수에 md5 가 없으면 본문의 md5 로 잇는다', () => {
        const score = scoreOf(NORMAL)
        expect(findTraceMismatch({ trace: NORMAL, pathHash: NORMAL.chart.sha256, score })).toBeNull()
        expect(findTraceMismatch({ trace: NORMAL, pathHash: NORMAL.chart.md5, score: { ...score, chartMd5: null } })).toBeNull()
        expect(findTraceMismatch({ trace: NORMAL, pathHash: 'a'.repeat(32), score })?.field).toBe('hash')
        expect(findTraceMismatch({ trace: NORMAL, pathHash: NORMAL.chart.md5, score: { ...score, chartSha256: 'b'.repeat(64) } })?.field).toBe(
            'chart.sha256',
        )
    })
})

describe('traceQuotaLevelOf — 보관 상한(ADR Q17)', () => {
    test('어느 한쪽이 90% 에 닿으면 near, 넘으면 over, 그 밖은 ok 다', () => {
        expect(traceQuotaLevelOf({ count: 0, bytes: 0 })).toBe(TRACE_QUOTA_LEVEL.OK)
        expect(traceQuotaLevelOf({ count: TRACE_USER_QUOTA.COUNT_MAX * TRACE_USER_QUOTA.NEAR_RATIO, bytes: 0 })).toBe(TRACE_QUOTA_LEVEL.NEAR)
        expect(traceQuotaLevelOf({ count: TRACE_USER_QUOTA.COUNT_MAX, bytes: 0 })).toBe(TRACE_QUOTA_LEVEL.NEAR)
        expect(traceQuotaLevelOf({ count: TRACE_USER_QUOTA.COUNT_MAX + 1, bytes: 0 })).toBe(TRACE_QUOTA_LEVEL.OVER)
        expect(traceQuotaLevelOf({ count: 1, bytes: TRACE_USER_QUOTA.TOTAL_BYTES_MAX + 1 })).toBe(TRACE_QUOTA_LEVEL.OVER)
    })
})

describe('traceService.upload', () => {
    test('저장한 data 는 본문 JSON 의 gzip 이고 크기·노트 수·점수 시각을 행에 남긴다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL)])
        const { service } = createMemoryTraceService(memory)
        const result = await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        expect(result).toMatchObject({ outcome: TRACE_UPLOAD_OUTCOME.CREATED, id: 'tr_1', analysis: TRACE_ANALYSIS_STATE.STORED })
        const [row] = [...memory.traces.values()]
        expect(JSON.parse(gunzipSync(row.data).toString('utf8'))).toEqual(NORMAL)
        expect(row).toMatchObject({
            scoreId: NORMAL.score_id,
            noteCount: NORMAL.notes.time_us.length,
            byteSize: JSON.stringify(NORMAL).length,
            storedSize: row.data.length,
        })
        expect(row.storedSize).toBeLessThan(row.byteSize)
        expect(memory.analyses.get('tr_1')).toMatchObject({ analysisVersion: PLAY_ANALYSIS_VERSION, patternJoin: 'absent' })
    })

    test('다른 사용자의 점수에 붙인 trace 는 없는 점수와 같게 거절하고 아무것도 저장하지 않는다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL, { userId: OTHER_USER_ID })])
        const { service } = createMemoryTraceService(memory)
        expect(await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })).toEqual({
            outcome: TRACE_UPLOAD_OUTCOME.SCORE_NOT_FOUND,
        })
        expect(memory.traces.size).toBe(0)
    })

    test('합계가 어긋나면 이유와 함께 거절하고 저장하지 않는다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL, { poor: 4 })])
        const { service } = createMemoryTraceService(memory)
        expect(await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })).toEqual({
            outcome: TRACE_UPLOAD_OUTCOME.MISMATCH,
            field: 'judge.poor',
            reason: 'score 4, trace 3',
        })
        expect(memory.traces.size).toBe(0)
    })

    test('같은 점수의 두 번째 업로드는 본문이 달라도 처음 id 를 돌려주고 행을 늘리지 않는다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL)])
        const { service } = createMemoryTraceService(memory)
        const first = await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        const again = await service.upload({ input: { ...NORMAL, lntype: 2 }, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        expect(again).toMatchObject({ outcome: TRACE_UPLOAD_OUTCOME.EXISTING, id: first.outcome === TRACE_UPLOAD_OUTCOME.CREATED ? first.id : '' })
        expect(memory.traces.size).toBe(1)
    })

    test('동시에 들어온 같은 점수의 trace 에 진 쪽은 이긴 trace 의 id 를 돌려준다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL)])
        const { service } = createMemoryTraceService(memory)
        memory.hooks.beforeInsert = (row) => {
            memory.hooks.beforeInsert = null
            memory.traces.set('tr_winner', { ...row, id: 'tr_winner' } satisfies TraceInsertRow)
        }
        expect(await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })).toMatchObject({
            outcome: TRACE_UPLOAD_OUTCOME.EXISTING,
            id: 'tr_winner',
        })
        expect([...memory.traces.keys()]).toEqual(['tr_winner'])
    })

    test('분석이 실패해도 trace 는 저장하고 pending 으로 답하며 실패를 알린다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL)])
        const { service, failures } = createMemoryTraceService(memory, {
            loadPattern: async () => {
                throw new TypeError('pattern store down')
            },
        })
        expect(await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })).toMatchObject({
            outcome: TRACE_UPLOAD_OUTCOME.CREATED,
            analysis: TRACE_ANALYSIS_STATE.PENDING,
        })
        expect(memory.traces.size).toBe(1)
        expect(memory.analyses.size).toBe(0)
        expect(failures).toHaveLength(1)
    })

    test('보관 상한을 넘은 사용자의 trace 도 저장하고, 응답의 storage 가 over 를 알린다', async () => {
        const scores = Array.from({ length: 3 }, (_, index) => scoreOf({ ...NORMAL, score_id: `sc_${index}` }))
        const memory = createMemoryTraceDb(scores)
        const { service } = createMemoryTraceService(memory)
        for (const row of scores.slice(1)) {
            memory.traces.set(`tr_old_${row.id}`, {
                id: `tr_old_${row.id}`,
                scoreId: row.id,
                userId: TRACE_USER_ID,
                chartSha256: row.chartSha256,
                chartMd5: row.chartMd5,
                playedAt: row.playedAt,
                formatVersion: 1,
                noteCount: 0,
                byteSize: TRACE_USER_QUOTA.TOTAL_BYTES_MAX / 2,
                storedSize: 0,
                data: Buffer.alloc(0),
                createdAt: 0,
            })
        }
        const result = await service.upload({ input: { ...NORMAL, score_id: 'sc_0' }, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        expect(result).toMatchObject({ outcome: TRACE_UPLOAD_OUTCOME.CREATED, storage: { traces: 3, level: TRACE_QUOTA_LEVEL.OVER } })
        expect(memory.traces.size).toBe(3)
        expect(await service.usageOf(TRACE_USER_ID)).toEqual(
            toTraceStorage({ count: 3, bytes: TRACE_USER_QUOTA.TOTAL_BYTES_MAX + JSON.stringify({ ...NORMAL, score_id: 'sc_0' }).length }),
        )
    })
})

describe('traceService.readAnalysis', () => {
    test('본인 trace 만 읽고, 분석이 없거나 낮은 버전이면 저장된 trace 에서 다시 계산해 저장한다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL)])
        const { service } = createMemoryTraceService(memory)
        await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        const stored = memory.analyses.get('tr_1')
        expect(await service.readAnalysis({ traceId: 'tr_1', userId: OTHER_USER_ID })).toBeNull()
        expect<unknown>(await service.readAnalysis({ traceId: 'tr_1', userId: TRACE_USER_ID })).toEqual(stored?.summary ?? null)

        memory.analyses.delete('tr_1')
        expect<unknown>(await service.readAnalysis({ traceId: 'tr_1', userId: TRACE_USER_ID })).toEqual(stored?.summary ?? null)
        expect(memory.analyses.has('tr_1')).toBe(true)

        if (stored) memory.analyses.set('tr_1', { ...stored, analysisVersion: PLAY_ANALYSIS_VERSION - 1, computedAt: 0 })
        await service.readAnalysis({ traceId: 'tr_1', userId: TRACE_USER_ID })
        expect(memory.analyses.get('tr_1')?.analysisVersion).toBe(PLAY_ANALYSIS_VERSION)
    })
})

describe('traceService.adminUsage', () => {
    test('합계, 바이트가 큰 사용자, near·over 사용자 수, 분석이 없는 trace 수를 센다', async () => {
        const memory = createMemoryTraceDb([scoreOf(NORMAL)], { [TRACE_USER_ID]: 'tracer' })
        const { service } = createMemoryTraceService(memory, {
            loadPattern: async () => {
                throw new TypeError('down')
            },
        })
        await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        const usage = await service.adminUsage()
        expect(usage).toMatchObject({
            totals: { users: 1, traces: 1 },
            accounts_near: 0,
            accounts_over: 0,
            top: [{ login_id: 'tracer', traces: 1, level: TRACE_QUOTA_LEVEL.OK }],
            analysis: { analysis_version: PLAY_ANALYSIS_VERSION, missing: 1, stale: 0 },
        })
    })
})

describe('traceService — 판별 분석 읽기(W5)', () => {
    const uploadMany = async (count: number) => {
        const scores = Array.from({ length: count }, (_, index) => scoreOf({ ...NORMAL, score_id: `sc_${index}` }, { playedAt: index }))
        const memory = createMemoryTraceDb(scores)
        const patternLoads = { count: 0 }
        const { service, failures } = createMemoryTraceService(memory, {
            loadPattern: async () => {
                patternLoads.count += 1
                return null
            },
        })
        for (const score of scores)
            await service.upload({ input: { ...NORMAL, score_id: score.id }, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
        return { memory, service, failures, patternLoads }
    }

    const breakTrace = (memory: ReturnType<typeof createMemoryTraceDb>, traceId: string) => {
        const row = memory.traces.get(traceId)
        if (row) memory.traces.set(traceId, { ...row, data: Buffer.from('not gzip') })
    }

    test('최근 판을 최신순으로 읽고, 낮은 분석 버전은 한 번에 정해진 수만 다시 계산하며 나머지는 stale 로 센다', async () => {
        const { memory, service } = await uploadMany(PLAY_ANALYSIS_READ_RECOMPUTE_MAX + 2)
        for (const [id, row] of memory.analyses) memory.analyses.set(id, { ...row, analysisVersion: PLAY_ANALYSIS_VERSION - 1 })
        const { plays, stale } = await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(plays).toHaveLength(PLAY_ANALYSIS_READ_RECOMPUTE_MAX)
        expect(stale).toBe(2)
        expect(plays.map((play) => play.playedAt)).toEqual(plays.map((play) => play.playedAt).toSorted((a, b) => b - a))
        expect(plays.every((play) => play.summary.analysis_version === PLAY_ANALYSIS_VERSION)).toBe(true)
        expect(await service.recentAnalyzedPlays({ userId: OTHER_USER_ID, limit: 100 })).toEqual({ plays: [], stale: 0, failed: 0 })
    })

    test('관리자 재계산은 id 순서로 limit 만큼 낮은 버전을 다시 계산하고, 이어 부를 자리를 돌려준다', async () => {
        const { memory, service } = await uploadMany(3)
        for (const [id, row] of memory.analyses) memory.analyses.set(id, { ...row, analysisVersion: PLAY_ANALYSIS_VERSION - 1 })
        const first = await service.recomputeStale({ limit: 2, after: null })
        expect(first).toMatchObject({
            analysis_version: PLAY_ANALYSIS_VERSION,
            examined: 2,
            recomputed: 2,
            skipped: [],
            next_after: 'tr_2',
            remaining: 1,
        })
        const second = await service.recomputeStale({ limit: 2, after: first.next_after })
        expect(second).toMatchObject({ examined: 1, next_after: 'tr_3', remaining: 0 })
        expect(await service.recomputeStale({ limit: 2, after: second.next_after })).toMatchObject({ examined: 0, next_after: null })
    })

    test('읽을 수 없는 trace 는 skipped 에 적고 다음 호출이 되풀이하지 않는다', async () => {
        const { memory, service } = await uploadMany(1)
        memory.analyses.clear()
        const row = memory.traces.get('tr_1')
        if (row) memory.traces.set('tr_1', { ...row, data: Buffer.from('not gzip') })
        const result = await service.recomputeStale({ limit: 5, after: null })
        expect(result).toMatchObject({ examined: 1, recomputed: 0, skipped: ['tr_1'], next_after: 'tr_1', remaining: 0 })
        expect(await service.recomputeStale({ limit: 5, after: result.next_after })).toMatchObject({ examined: 0, remaining: 0 })
        expect(await service.recomputeStale({ limit: 5, after: null })).toMatchObject({ examined: 0, skipped: [] })
    })

    test('다시 계산이 실패한 trace 하나만 실패로 표시하고 나머지로 답하며, 다음 읽기는 그 trace 를 다시 계산하지 않는다', async () => {
        const { memory, service, failures } = await uploadMany(3)
        memory.analyses.clear()
        breakTrace(memory, 'tr_2')
        const first = await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(first.plays.map((play) => play.scoreId).toSorted()).toEqual(['sc_0', 'sc_2'])
        expect([first.stale, first.failed]).toEqual([0, 1])
        expect(failures).toHaveLength(1)
        expect(memory.analyses.get('tr_2')).toMatchObject({
            analysisVersion: PLAY_ANALYSIS_VERSION,
            patternJoin: 'failed',
            summary: { failed: true, analysis_version: PLAY_ANALYSIS_VERSION, pattern_analysis_version: ANALYSIS_VERSION },
        })

        const second = await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect([second.plays.length, second.stale, second.failed]).toEqual([2, 0, 1])
        expect(failures).toHaveLength(1)
        expect(await service.adminUsage()).toMatchObject({ analysis: { missing: 0, stale: 0 } })
    })

    test('한 판을 읽는 경로도 실패한 trace 에서 던지지 않고 없는 판처럼 null 이다', async () => {
        const { memory, service } = await uploadMany(1)
        memory.analyses.clear()
        breakTrace(memory, 'tr_1')
        expect(await service.analyzedPlayOfScore({ userId: TRACE_USER_ID, scoreId: 'sc_0' })).toBeNull()
        expect(await service.readAnalysis({ traceId: 'tr_1', userId: TRACE_USER_ID })).toBeNull()
        expect(
            await service.bestAnalyzedPlayOnChart({
                userId: TRACE_USER_ID,
                chartSha256: NORMAL.chart.sha256,
                scratchAuto: false,
                excludeScoreId: null,
            }),
        ).toBeNull()
    })

    test('실패 표시는 분석 버전이 오르면 풀리고 그때 한 번 다시 계산한다', async () => {
        const { memory, service, failures } = await uploadMany(1)
        memory.analyses.clear()
        breakTrace(memory, 'tr_1')
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        const marker = memory.analyses.get('tr_1')
        expect(failures).toHaveLength(1)

        if (marker) memory.analyses.set('tr_1', { ...marker, analysisVersion: PLAY_ANALYSIS_VERSION - 1 })
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(failures).toHaveLength(2)

        if (marker) memory.analyses.set('tr_1', { ...marker, summary: { ...marker.summary, pattern_analysis_version: ANALYSIS_VERSION - 1 } })
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(failures).toHaveLength(3)
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(failures).toHaveLength(3)
    })

    test('실패 표시를 저장하지 못해도 읽기는 던지지 않는다', async () => {
        const { memory, service } = await uploadMany(2)
        memory.analyses.clear()
        breakTrace(memory, 'tr_2')
        const saveFailure = memory.db.saveAnalysisFailure
        memory.db.saveAnalysisFailure = async () => {
            throw new Error('database is down')
        }
        const result = await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(result.plays.map((play) => play.scoreId)).toEqual(['sc_0'])
        expect(result.failed).toBe(1)
        memory.db.saveAnalysisFailure = saveFailure
    })

    test('판 분석 버전과 곡별 분류 버전 중 하나라도 낮은 분석은 읽을 때 다시 계산하고, 둘 다 현재면 그대로 둔다', async () => {
        const { memory, service, patternLoads } = await uploadMany(1)
        const current = memory.analyses.get('tr_1')
        const loadsAfterUpload = patternLoads.count
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(patternLoads.count).toBe(loadsAfterUpload)

        if (!current) throw new Error('the upload stores an analysis')
        const olderPattern = { ...current, summary: { ...current.summary, pattern_analysis_version: ANALYSIS_VERSION - 1 } }
        memory.analyses.set('tr_1', olderPattern)
        const { plays } = await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(patternLoads.count).toBe(loadsAfterUpload + 1)
        expect(plays[0].summary.pattern_analysis_version).toBe(ANALYSIS_VERSION)
        expect(memory.analyses.get('tr_1')?.summary.pattern_analysis_version).toBe(ANALYSIS_VERSION)

        const withoutPatternVersion = Object.fromEntries(Object.entries(current.summary).filter(([key]) => key !== 'pattern_analysis_version'))
        memory.analyses.set('tr_1', { ...current, summary: withoutPatternVersion })
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(patternLoads.count).toBe(loadsAfterUpload + 2)

        memory.analyses.set('tr_1', { ...current, analysisVersion: PLAY_ANALYSIS_VERSION - 1 })
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(patternLoads.count).toBe(loadsAfterUpload + 3)

        memory.analyses.set('tr_1', { ...current, summary: { ...current.summary, analysis_version: PLAY_ANALYSIS_VERSION - 1 } })
        await service.recentAnalyzedPlays({ userId: TRACE_USER_ID, limit: 100 })
        expect(patternLoads.count).toBe(loadsAfterUpload + 4)
    })

    test('관리자 수치와 일괄 재계산도 곡별 분류 버전이 낮은 분석을 뒤처진 것으로 센다', async () => {
        const { memory, service } = await uploadMany(2)
        const current = memory.analyses.get('tr_2')
        if (!current) throw new Error('the upload stores an analysis')
        memory.analyses.set('tr_2', { ...current, summary: { ...current.summary, pattern_analysis_version: ANALYSIS_VERSION - 1 } })
        expect(await service.adminUsage()).toMatchObject({
            analysis: { analysis_version: PLAY_ANALYSIS_VERSION, pattern_analysis_version: ANALYSIS_VERSION, missing: 0, stale: 1 },
        })
        const result = await service.recomputeStale({ limit: 10, after: null })
        expect(result).toMatchObject({ examined: 1, recomputed: 1, skipped: [], remaining: 0, pattern_analysis_version: ANALYSIS_VERSION })
        expect(memory.analyses.get('tr_2')?.summary.pattern_analysis_version).toBe(ANALYSIS_VERSION)
    })

    test('공유 설정은 행이 없으면 꺼짐이고, 켠 값을 그 시각과 함께 저장한다', async () => {
        const { service } = await uploadMany(0)
        expect(await service.shareSettingOf(TRACE_USER_ID)).toEqual({ rival_segments: false, updated_at: null })
        expect(await service.putShareSetting({ userId: TRACE_USER_ID, rivalSegments: true })).toEqual({
            rival_segments: true,
            updated_at: 1_791_970_080_000,
        })
        expect(await service.shareSettingOf(TRACE_USER_ID)).toMatchObject({ rival_segments: true })
    })

    test('한쪽만 라이벌로 등록했으면 공유를 켜도 구간 요약을 내보내지 않는다', async () => {
        const { memory, service } = await uploadMany(1)
        const params = { requesterId: OTHER_USER_ID, rivalId: TRACE_USER_ID, chartSha256: NORMAL.chart.sha256, scratchAuto: false }
        await service.putShareSetting({ userId: TRACE_USER_ID, rivalSegments: true })
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        memory.rivals.add(`${OTHER_USER_ID}>${TRACE_USER_ID}`)
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        memory.rivals.delete(`${OTHER_USER_ID}>${TRACE_USER_ID}`)
        memory.rivals.add(`${TRACE_USER_ID}>${OTHER_USER_ID}`)
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
    })

    test('서로 라이벌이어도 공유를 끈 사용자의 구간 요약은 내보내지 않는다', async () => {
        const { memory, service } = await uploadMany(1)
        const params = { requesterId: OTHER_USER_ID, rivalId: TRACE_USER_ID, chartSha256: NORMAL.chart.sha256, scratchAuto: false }
        memory.rivals.add(`${OTHER_USER_ID}>${TRACE_USER_ID}`)
        memory.rivals.add(`${TRACE_USER_ID}>${OTHER_USER_ID}`)
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
        await service.putShareSetting({ userId: TRACE_USER_ID, rivalSegments: false })
        expect(await service.rivalSegmentsOnChart(params)).toEqual({ allowed: false })
    })

    test('라이벌의 구간 요약은 서로 라이벌이고 그 라이벌이 허용했을 때만, 구간 수치만 내보낸다', async () => {
        const { memory, service } = await uploadMany(1)
        const params = { requesterId: OTHER_USER_ID, rivalId: TRACE_USER_ID, chartSha256: NORMAL.chart.sha256, scratchAuto: false }
        memory.rivals.add(`${OTHER_USER_ID}>${TRACE_USER_ID}`)
        memory.rivals.add(`${TRACE_USER_ID}>${OTHER_USER_ID}`)
        await service.putShareSetting({ userId: TRACE_USER_ID, rivalSegments: true })
        const shared = await service.rivalSegmentsOnChart(params)
        expect(shared.allowed).toBe(true)
        const play = shared.allowed ? shared.play : null
        expect(Object.keys(play?.segments[0] ?? {}).toSorted()).toEqual(['bp', 'chart_notes', 'ex_score', 'id', 'notes'])
        expect(JSON.stringify(play)).not.toContain('err_us')
        expect(await service.rivalSegmentsOnChart({ ...params, scratchAuto: true })).toEqual({ allowed: true, play: null })
    })
})
