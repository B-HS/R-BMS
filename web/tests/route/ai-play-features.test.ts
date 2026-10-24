import { beforeEach, describe, expect, test } from 'bun:test'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAiPlayFeatureRoutes, type AiPlayFeatureRouteDeps } from '@server/route/ai'
import { createAiGaugeAdviceService } from '@server/service/domain/ai/gauge-advice.service'
import { createAiPlayCompareService } from '@server/service/domain/ai/play-compare.service'
import { createAiPlayReportService } from '@server/service/domain/ai/play-report.service'
import { createAiReplayCoachService, type PlayScoreRow } from '@server/service/domain/ai/replay-coach.service'
import { countTraceJudgements } from '@server/service/domain/trace/trace-check'
import { TRACE_USER_ID, createMemoryTraceDb, createMemoryTraceService, readContractTrace, scoreRowFor } from '../support/trace-fixture'

const NORMAL = readContractTrace('trace-normal.json')
const ALICE: AuthUser = { id: TRACE_USER_ID, loginId: 'alice', name: 'Alice', role: 'user' }
const ORIGIN = 'http://localhost:3000'
const BASE = '/api/fe/me/ai'

const playRow: PlayScoreRow = {
    scoreId: NORMAL.score_id,
    playedAt: 1,
    chartSha256: NORMAL.chart.sha256,
    chartMd5: NORMAL.chart.md5,
    title: 'Chart',
    artist: 'Artist',
    clear: 4,
    exScore: 41,
    maxExScore: 64,
    minbp: 3,
    random: 'Off',
    gauge: 2,
    scratchAuto: false,
    pgreat: 15,
    great: 11,
    good: 3,
    bad: 0,
    poor: 3,
}

const createRoutes = async (options: { withTrace: boolean }) => {
    const memory = createMemoryTraceDb([scoreRowFor(NORMAL, countTraceJudgements(NORMAL))])
    const { service } = createMemoryTraceService(memory)
    if (options.withTrace) await service.upload({ input: NORMAL, pathHash: NORMAL.chart.md5, userId: TRACE_USER_ID })
    const runs: unknown[] = []
    const shared = {
        plays: service,
        runner: {
            run: async (input: unknown) => {
                runs.push(input)
                return { ok: false as const, reason: 'credential_not_found' as const }
            },
        },
        results: { findLatest: async () => null },
    }
    const ownScore = {
        findOwnedPlayScore: async ({ userId, scoreId }: { userId: string; scoreId: string }) =>
            userId === ALICE.id && scoreId === playRow.scoreId ? playRow : null,
    }
    const deps: AiPlayFeatureRouteDeps = {
        authService: { resolveSessionUser: async (request) => (request.headers.get('x-session-user') === ALICE.loginId ? ALICE : null) },
        aiPlayReportService: createAiPlayReportService({
            ...shared,
            db: { sumRecentFastSlow: async () => ({ plays: 0, fast: 0, slow: 0 }) },
            loadTable: () => null,
            loadRecords: async () => null,
            loadChartTags: async () => ({ shares: new Map(), staleMd5s: new Set() }),
        }),
        aiReplayCoachService: createAiReplayCoachService({ ...shared, db: ownScore }),
        aiPlayCompareService: createAiPlayCompareService({
            ...shared,
            db: { ...ownScore, findPublicBest: async () => null, listRivals: async () => [], findLoginId: async () => ALICE.loginId },
        }),
        aiGaugeAdviceService: createAiGaugeAdviceService({ ...shared, db: { listRecentGaugeRows: async () => [] }, levelLabelOf: () => null }),
    }
    return {
        routes: createAiPlayFeatureRoutes(
            () => deps,
            async () => undefined,
            async () => 'ko' as const,
        ),
        runs,
        memory,
    }
}

const request = (path: string, init: { method?: string; user?: AuthUser; bearer?: string; body?: unknown } = {}) =>
    new Request(`${ORIGIN}${BASE}${path}`, {
        method: init.method ?? 'GET',
        headers: {
            'content-type': 'application/json',
            ...(init.user ? { 'x-session-user': init.user.loginId } : {}),
            ...(init.bearer ? { authorization: `Bearer ${init.bearer}` } : {}),
        },
        body: init.body === undefined ? undefined : JSON.stringify(init.body),
    })

const scoreContext = (scoreId: string) => ({ params: Promise.resolve({ scoreId }) })

beforeEach(() => resetRateLimits())

describe('판별 기반 AI 기능 API', () => {
    test('세션이 없거나 API 토큰만 있으면 401 이고 공급자를 부르지 않는다', async () => {
        const { routes, runs } = await createRoutes({ withTrace: true })
        for (const init of [{}, { bearer: 'rbms_token' }]) {
            expect((await routes.reportView(request('/report', init), undefined)).status).toBe(401)
            expect((await routes.reportInterpret(request('/report', { ...init, method: 'POST' }), undefined)).status).toBe(401)
            expect((await routes.coachView(request('/plays/x/coach', init), scoreContext(NORMAL.score_id))).status).toBe(401)
            expect(
                (await routes.compareInterpret(request('/plays/x/compare', { ...init, method: 'POST', body: {} }), scoreContext(NORMAL.score_id)))
                    .status,
            ).toBe(401)
            expect((await routes.gaugeAdviceView(request('/gauge-advice', init), undefined)).status).toBe(401)
        }
        expect(runs).toEqual([])
    })

    test('보기는 결정론 결과만 돌려주고 캐시하지 않으며, 본인 판이 아니면 없는 점수와 같다', async () => {
        const { routes, runs } = await createRoutes({ withTrace: true })
        const coach = await routes.coachView(request(`/plays/${NORMAL.score_id}/coach`, { user: ALICE }), scoreContext(NORMAL.score_id))
        expect(coach.status).toBe(200)
        expect(coach.headers.get('cache-control')).toBe('private, no-store')
        const body = await coach.json()
        expect(body.data).toMatchObject({ play: { score_id: NORMAL.score_id }, notice: null, ai: null })
        expect(body.data.analysis.segments).toHaveLength(32)
        const other = await routes.coachView(request('/plays/sc_other/coach', { user: ALICE }), scoreContext('sc_other'))
        expect([other.status, (await other.json()).error.code]).toEqual([404, 'IR_SCORE_NOT_FOUND'])
        expect((await routes.coachView(request('/plays/bad id/coach', { user: ALICE }), scoreContext('bad id'))).status).toBe(400)
        expect(runs).toEqual([])
    })

    test('판별 데이터가 없는 판의 코치와 표본이 모자란 리포트·게이지 조언은 422 AI_NOT_ENOUGH_DATA 이고 AI 를 부르지 않는다', async () => {
        const { routes, runs } = await createRoutes({ withTrace: false })
        const coach = await routes.coachInterpret(
            request(`/plays/${NORMAL.score_id}/coach`, { user: ALICE, method: 'POST' }),
            scoreContext(NORMAL.score_id),
        )
        expect([coach.status, (await coach.json()).error]).toEqual([
            422,
            expect.objectContaining({ code: 'AI_NOT_ENOUGH_DATA', details: expect.objectContaining({ need: 'play_trace' }) }),
        ])
        const report = await routes.reportInterpret(request('/report', { user: ALICE, method: 'POST' }), undefined)
        expect(report.status).toBe(422)
        const gauge = await routes.gaugeAdviceInterpret(request('/gauge-advice', { user: ALICE, method: 'POST' }), undefined)
        expect(gauge.status).toBe(422)
        const compare = await routes.compareInterpret(
            request(`/plays/${NORMAL.score_id}/compare`, { user: ALICE, method: 'POST', body: { against: 'best' } }),
            scoreContext(NORMAL.score_id),
        )
        expect([compare.status, (await compare.json()).error.details.need]).toEqual([422, 'comparable_play'])
        expect(runs).toEqual([])
    })

    test('저장된 trace 하나를 읽을 수 없어도 보기는 500 이 아니라 그 판을 뺀 결과로 답하고, 같은 판을 다시 계산하지 않는다', async () => {
        const { routes, runs, memory } = await createRoutes({ withTrace: true })
        memory.analyses.clear()
        const traceId = [...memory.traces.keys()][0]
        const stored = memory.traces.get(traceId)
        if (stored) memory.traces.set(traceId, { ...stored, data: Buffer.from('not gzip') })
        const scoreId = NORMAL.score_id

        for (const round of [1, 2]) {
            const coach = await routes.coachView(request(`/plays/${scoreId}/coach`, { user: ALICE }), scoreContext(scoreId))
            expect([round, coach.status]).toEqual([round, 200])
            expect((await coach.json()).data).toMatchObject({ analysis: null, notice: { need: 'play_trace' } })
            const report = await routes.reportView(request('/report', { user: ALICE }), undefined)
            expect([round, report.status]).toEqual([round, 200])
            expect((await report.json()).data).toMatchObject({ plays: { analyzed: 0, measured: 0, failed: 1 }, notice: { need: 'traced_plays' } })
            const compare = await routes.compareView(request(`/plays/${scoreId}/compare?against=best`, { user: ALICE }), scoreContext(scoreId))
            expect([round, compare.status]).toEqual([round, 200])
            const gauge = await routes.gaugeAdviceView(request('/gauge-advice', { user: ALICE }), undefined)
            expect([round, gauge.status]).toEqual([round, 200])
        }
        expect(memory.analyses.get(traceId)).toMatchObject({ patternJoin: 'failed' })
        expect(runs).toEqual([])
    })

    test('비교 대상은 best·rival 만 받고, 라이벌 아이디 꼴이 틀리면 400 이다', async () => {
        const { routes } = await createRoutes({ withTrace: true })
        const path = `/plays/${NORMAL.score_id}/compare`
        expect((await routes.compareView(request(`${path}?against=friend`, { user: ALICE }), scoreContext(NORMAL.score_id))).status).toBe(400)
        expect((await routes.compareView(request(`${path}?against=rival&rival=a%20b`, { user: ALICE }), scoreContext(NORMAL.score_id))).status).toBe(
            400,
        )
        const view = await routes.compareView(request(`${path}?against=rival&rival=nobody`, { user: ALICE }), scoreContext(NORMAL.score_id))
        expect((await view.json()).data).toMatchObject({ mode: 'none', reason: 'not_a_rival' })
    })
})
