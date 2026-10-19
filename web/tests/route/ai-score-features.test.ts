import { beforeEach, describe, expect, test } from 'bun:test'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createAiScoreFeatureRoutes, type AiScoreFeatureRouteDeps } from '@server/route/ai'
import { createAiJudgeHabitService } from '@server/service/domain/ai/judge-habit.service'
import { createAiWeeklySummaryService } from '@server/service/domain/ai/weekly-summary.service'
import { SCORE_API_KEY, SCORE_USER, levelRecords, makeTable, md5Of, recordOf, setupScoreFeatures } from '../support/ai-score-fixture'

const ALICE: AuthUser = { id: SCORE_USER, loginId: 'alice', name: 'Alice', role: 'user' }
const ORIGIN = 'http://localhost:3000'
const BASE = '/api/fe/me/ai'

const createRoutes = () => {
    const table = makeTable()
    const context = setupScoreFeatures({ table, records: { ...levelRecords(table, { '1': 'Hard' }), [md5Of(200)]: recordOf('Failed') } })
    const seenUserIds: string[] = []
    const deps: AiScoreFeatureRouteDeps = {
        authService: { resolveSessionUser: async (request) => (request.headers.get('x-session-user') === ALICE.loginId ? ALICE : null) },
        aiRecommendService: context.recommend,
        aiPracticeFolderService: context.practice,
        aiGoalService: context.goals,
        aiWeeklySummaryService: createAiWeeklySummaryService({
            db: {
                listScoresBetween: async ({ userId }) => {
                    seenUserIds.push(userId)
                    return []
                },
                listPriorBests: async () => [],
            },
            runner: context.aiFeatureRunner,
            results: context.aiResultService,
            levelLabelOf: () => null,
            now: () => Date.UTC(2026, 9, 21),
        }),
        aiJudgeHabitService: createAiJudgeHabitService({
            db: {
                listRecentTimingRows: async ({ userId }) => {
                    seenUserIds.push(userId)
                    return []
                },
            },
            runner: context.aiFeatureRunner,
            results: context.aiResultService,
        }),
    }
    return {
        routes: createAiScoreFeatureRoutes(
            () => deps,
            async () => undefined,
        ),
        context,
        seenUserIds,
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

const goalContext = (goalId: string) => ({ params: Promise.resolve({ goalId }) })

beforeEach(() => resetRateLimits())

describe('점수 기반 AI 기능 API', () => {
    test('세션이 없거나 API 토큰만 있으면 401 이고 아무것도 읽지 않는다', async () => {
        const { routes, context, seenUserIds } = createRoutes()
        for (const init of [{}, { bearer: 'rbms_token' }]) {
            expect((await routes.recommendView(request('/recommend', init), undefined)).status).toBe(401)
            expect((await routes.recommendInterpret(request('/recommend', { ...init, method: 'POST', body: {} }), undefined)).status).toBe(401)
            expect((await routes.weeklyView(request('/weekly', init), undefined)).status).toBe(401)
            expect((await routes.judgeHabitInterpret(request('/judge-habit', { ...init, method: 'POST' }), undefined)).status).toBe(401)
            expect((await routes.goalList(request('/goals', init), undefined)).status).toBe(401)
            expect((await routes.practiceCreate(request('/practice-folder', { ...init, method: 'POST', body: {} }), undefined)).status).toBe(401)
        }
        expect(seenUserIds).toEqual([])
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('보기 응답은 비공개이고 결정론 결과를 담으며 AI 를 부르지 않는다', async () => {
        const { routes, context } = createRoutes()
        const response = await routes.recommendView(request('/recommend', { user: ALICE }), undefined)
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        expect((await response.json()).data).toMatchObject({ table: { id: 'insane1' }, frontier: { level: '2' }, ai: null })
        expect((await routes.recommendView(request('/recommend?table=missing', { user: ALICE }), undefined)).status).toBe(404)
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('표본이 모자라면 AI 를 부르지 않고 422 AI_NOT_ENOUGH_DATA 와 필요한 것을 알려 준다', async () => {
        const { routes, context } = createRoutes()
        await context.aiCredentialService.register({ userId: ALICE.id, provider: 'anthropic', apiKey: SCORE_API_KEY })
        await context.aiCredentialService.setModel({ userId: ALICE.id, provider: 'anthropic', model: 'claude-test' })
        for (const [handler, path, body] of [
            [routes.weeklyInterpret, '/weekly', {}],
            [routes.judgeHabitInterpret, '/judge-habit', undefined],
        ] as const) {
            const response = await handler(request(path, { user: ALICE, method: 'POST', body }), undefined)
            expect(response.status).toBe(422)
            expect((await response.json()).error).toMatchObject({ code: 'AI_NOT_ENOUGH_DATA', details: { reason: 'not_enough_data' } })
        }
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('키가 없으면 AI_CREDENTIAL_NOT_FOUND 이고 잘못된 주·주 아닌 날짜는 400 이다', async () => {
        const { routes } = createRoutes()
        const missing = await routes.recommendInterpret(request('/recommend', { user: ALICE, method: 'POST', body: {} }), undefined)
        expect([missing.status, (await missing.json()).error.code]).toEqual([404, 'AI_CREDENTIAL_NOT_FOUND'])
        expect((await routes.weeklyView(request('/weekly?week=2026-10-20', { user: ALICE }), undefined)).status).toBe(400)
        expect((await routes.weeklyView(request('/weekly?week=yesterday', { user: ALICE }), undefined)).status).toBe(400)
    })

    test('목표 만들기 201·잘못된 목표 400·다른 사용자 목표 404·보관 204', async () => {
        const { routes } = createRoutes()
        const created = await routes.goalCreate(
            request('/goals', {
                user: ALICE,
                method: 'POST',
                body: { kind: 'lamp_count_at_level', table: 'insane1', level: '2', lamp: 'Hard', count: 2 },
            }),
            undefined,
        )
        expect(created.status).toBe(201)
        const goal = (await created.json()).data
        expect(goal).toMatchObject({ params: { kind: 'lamp_count_at_level' }, progress: { label: '★2 에서 Hard 이상 2곡', achieved: 0 } })
        const invalid = await routes.goalCreate(
            request('/goals', {
                user: ALICE,
                method: 'POST',
                body: { kind: 'lamp_count_at_level', table: 'insane1', level: '2', lamp: 'Hard', count: 9 },
            }),
            undefined,
        )
        expect([invalid.status, (await invalid.json()).error.details]).toEqual([400, { field: 'count', reason: 'invalid_goal' }])
        expect(
            (await routes.goalCreate(request('/goals', { user: ALICE, method: 'POST', body: { kind: 'free_text', text: 'x' } }), undefined)).status,
        ).toBe(400)
        const plan = await routes.goalPlanView(request(`/goals/${goal.id}/plan`, { user: ALICE }), goalContext(goal.id))
        expect((await plan.json()).data.remaining.map((chart: { md5: string }) => chart.md5)).toEqual([
            md5Of(200),
            md5Of(201),
            md5Of(202),
            md5Of(203),
        ])
        expect((await routes.goalPlanView(request('/goals/aig_x/plan', { user: ALICE }), goalContext('aig_x'))).status).toBe(400)
        expect((await routes.goalPlanView(request('/goals/x/plan', { user: ALICE }), goalContext(`aig_${'0'.repeat(30)}`))).status).toBe(404)
        expect((await routes.goalArchive(request(`/goals/${goal.id}`, { user: ALICE, method: 'DELETE' }), goalContext(goal.id))).status).toBe(204)
        expect((await routes.goalArchive(request(`/goals/${goal.id}`, { user: ALICE, method: 'DELETE' }), goalContext(goal.id))).status).toBe(404)
    })

    test('연습 폴더 만들기는 후보 안 곡으로만 201 이고, 후보가 하나도 없으면 400 이다', async () => {
        const { routes } = createRoutes()
        const body = { source: 'recommend', name_suffix: '실패곡', description: '', items: [{ md5: md5Of(200), memo: '집중' }] }
        const created = await routes.practiceCreate(request('/practice-folder', { user: ALICE, method: 'POST', body }), undefined)
        expect(created.status).toBe(201)
        expect((await created.json()).data).toMatchObject({ outcome: 'created', name: 'AI 연습 · 실패곡', item_count: 1 })
        const again = await routes.practiceCreate(request('/practice-folder', { user: ALICE, method: 'POST', body }), undefined)
        expect([again.status, (await again.json()).data.outcome]).toEqual([200, 'updated'])
        const outside = await routes.practiceCreate(
            request('/practice-folder', { user: ALICE, method: 'POST', body: { ...body, items: [{ md5: md5Of(999), memo: '' }] } }),
            undefined,
        )
        expect([outside.status, (await outside.json()).error.details.reason]).toEqual([400, 'no_candidate_items'])
        const view = await routes.practiceView(request('/practice-folder?source=tag&tag=trill', { user: ALICE }), undefined)
        expect((await view.json()).data).toMatchObject({ source: 'tag', tag: 'trill', notice: { need: 'chart_patterns' }, folders: { used: 1 } })
    })
})
