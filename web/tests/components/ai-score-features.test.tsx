import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { click, page, waitFor } = await import('../support/dom-harness')
const { AiRecommendPanel } = await import('../../src/widgets/ai/ai-recommend-panel')
const { AiWeeklyPanel } = await import('../../src/widgets/ai/ai-weekly-panel')
const { AiJudgeHabitPanel } = await import('../../src/widgets/ai/ai-judge-habit-panel')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')

type RecommendView = import('../../src/entities/ai.type').AiRecommendView
type WeeklyView = import('../../src/entities/ai.type').AiWeeklyView
type JudgeHabitView = import('../../src/entities/ai.type').AiJudgeHabitView

const RECOMMEND_PATH = '/api/fe/me/ai/recommend'
const WEEKLY_PATH = '/api/fe/me/ai/weekly'
const HABIT_PATH = '/api/fe/me/ai/judge-habit'
const USAGE_PATH = '/api/fe/me/ai/usage'
const CREATED_AT = Date.UTC(2026, 9, 20, 3)
const MD5 = { alpha: 'a'.repeat(32), bravo: 'b'.repeat(32) }

const candidate = (md5: string, title: string) => ({
    key: `chart.${md5}`,
    md5,
    title,
    artist: 'artist',
    level: '12',
    lamp: 'Failed' as const,
    assist_grade: 0 as const,
    ex_rate: 82.5,
    bp: 40,
    tags: [{ tag: 'trill', ratio: 0.3 }],
    kind: 'failed' as const,
    score: 3.8,
})

const recommendView = (ai: RecommendView['ai'] = null): RecommendView => ({
    table: { id: 'insane1', name: '発狂', symbol: '★' },
    frontier: { key: 'level.12', level: '12', charts: 40, played: 30, cleared: 12, hard_or_better: 3, clear_rate: 0.3 },
    levels: [
        { key: 'level.11', level: '11', charts: 40, played: 40, cleared: 30, hard_or_better: 10, clear_rate: 0.75 },
        { key: 'level.12', level: '12', charts: 40, played: 30, cleared: 12, hard_or_better: 3, clear_rate: 0.3 },
    ],
    candidates: [candidate(MD5.alpha, 'Alpha'), candidate(MD5.bravo, 'Bravo')],
    pattern_coverage: { with_features: 2, candidates: 2, stale_features: 0 },
    notice: null,
    ai,
})

const interpretation = <T,>(output: T) => ({
    output,
    cached: false,
    is_current: true,
    provider: 'anthropic' as const,
    model: 'claude-test',
    usage: { inputTokens: 900, outputTokens: 120, cachedInputTokens: 0 },
    created_at: CREATED_AT,
})

let restoreFetch: () => void = () => undefined

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

describe('다음 도전 곡 화면', () => {
    test('열면 결정론 후보만 그리고 AI 는 누를 때 한 번만 부르며, AI 순서와 이유를 후보 정보와 함께 보인다', async () => {
        const output = { summary: '★12 에 들어섰습니다.', picks: [{ md5: MD5.bravo, reason: '트릴이 많은 곡부터', focus_key: `chart.${MD5.bravo}` }] }
        const state = { stored: recommendView() }
        const installed = installFetchRoutes({
            [`GET ${RECOMMEND_PATH}`]: () => jsonResponse(state.stored),
            [`POST ${RECOMMEND_PATH}`]: () => {
                state.stored = recommendView({ ...interpretation(output), cached: true })
                return jsonResponse(interpretation(output))
            },
            [`GET ${USAGE_PATH}`]: () => jsonResponse(null),
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiRecommendPanel />)

        expect(await page.findByText('Alpha')).toBeTruthy()
        expect(document.querySelector('[aria-label="추천 후보"]')?.textContent).toContain('트릴 30%')
        expect(installed.calls.filter((call) => call.method === 'POST')).toHaveLength(0)

        await click(page.getByRole('button', { name: 'AI 해석 만들기' }))
        expect(await page.findByText('트릴이 많은 곡부터')).toBeTruthy()
        expect(page.getByText('★12 에 들어섰습니다.')).toBeTruthy()
        const posts = installed.calls.filter((call) => call.method === 'POST')
        expect(posts).toEqual([{ method: 'POST', path: RECOMMEND_PATH, body: { table: 'insane1' } }])
        const aiList = document.querySelector('[aria-label="AI 가 고른 순서"]')?.textContent
        expect(aiList).toContain('Bravo')
        expect(aiList).not.toContain('Alpha')
    })

    test('옛 분석 버전의 패턴을 쓴 후보가 있으면 몇 곡인지 알린다', async () => {
        const installed = installFetchRoutes({
            [`GET ${RECOMMEND_PATH}`]: () =>
                jsonResponse({ ...recommendView(), pattern_coverage: { with_features: 2, candidates: 2, stale_features: 1 } }),
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiRecommendPanel />)
        expect(await page.findByText(/옛 분석 버전 1곡/)).toBeTruthy()
    })

    test('표본이 모자라면 버튼이 막히고 이유를 보이며, 키가 없으면 설정으로 안내한다', async () => {
        const installed = installFetchRoutes({
            [`GET ${RECOMMEND_PATH}`]: () => jsonResponse({ ...recommendView(), notice: { need: 'table_records' } }),
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiRecommendPanel />)
        expect(await page.findByText(/난이도표 곡의 기록이 아직 없습니다/)).toBeTruthy()
        expect((page.getByRole('button', { name: 'AI 해석 만들기' }) as HTMLButtonElement).disabled).toBe(true)
        restoreFetch()

        await cleanupScreens()
        const second = installFetchRoutes({
            [`GET ${RECOMMEND_PATH}`]: () => jsonResponse(recommendView()),
            [`POST ${RECOMMEND_PATH}`]: () => errorResponse(404, 'AI_CREDENTIAL_NOT_FOUND', '등록된 AI 키가 없습니다. 설정에서 키를 등록해 주세요.'),
            [`GET ${USAGE_PATH}`]: () => jsonResponse(null),
        })
        restoreFetch = second.restore
        await renderWithQueryClient(<AiRecommendPanel />)
        await click(await page.findByRole('button', { name: 'AI 해석 만들기' }))
        expect(await page.findByRole('link', { name: 'AI 설정 열기' })).toBeTruthy()
    })
})

const totals = (plays: number): WeeklyView['totals'] => ({
    plays,
    charts: plays,
    active_days: 1,
    first_plays: 0,
    lamp_ups: 0,
    new_clears: 0,
    ex_ups: 0,
    fast: 0,
    slow: 0,
    fast_share: null,
    mean_offset_ms: null,
})

const weeklyView = (start: string, end: string, inProgress: boolean): WeeklyView => ({
    week: { start, end, in_progress: inProgress },
    totals: totals(3),
    previous: totals(1),
    lamp_changes: [],
    ex_changes: [],
    levels: [],
    notice: null,
    ai: null,
})

describe('주간 요약 화면', () => {
    test('이번 주는 진행 중으로 보이고 지난주 버튼은 그 주 월요일을 묻는다', async () => {
        const installed = installFetchRoutes({
            [`GET ${WEEKLY_PATH}`]: () => jsonResponse(weeklyView('2026-10-19', '2026-10-25', true)),
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiWeeklyPanel />)
        expect(await page.findByText('진행 중')).toBeTruthy()
        expect((page.getByRole('button', { name: '다음 주' }) as HTMLButtonElement).disabled).toBe(true)
        const before = installed.calls.length
        await click(page.getByRole('button', { name: '지난주' }))
        await waitFor(() => expect(installed.calls.length).toBeGreaterThan(before))
        expect(installed.calls.at(-1)).toMatchObject({ method: 'GET', path: WEEKLY_PATH })
        expect(installed.calls.filter((call) => call.method === 'POST')).toHaveLength(0)
    })
})

const habitView = (): JudgeHabitView => ({
    plays: 12,
    from: CREATED_AT,
    to: CREATED_AT,
    fast_slow: { fast: 840, slow: 360, fast_share: 0.7, grade: 'strong', leaning: 'fast', by_grade: [{ grade: 'great', early: 500, late: 200 }] },
    offset: {
        plays: 12,
        notes: 10_800,
        mean_offset_ms: 4.4,
        grade: 'strong',
        direction: 'later',
        suggested_shift_ms: -4,
        auto_adjust_plays: 0,
        source: 'score',
    },
    trace_timing: { plays: 0, notes: 0, mean_ms: null, ci_low_ms: null, ci_high_ms: null, grade: 'insufficient', lanes: [] },
    avgjudge: { plays: 12, older_ms: 14, newer_ms: 10, change_ms: -4, grade: 'strong' },
    green: {
        plays: 12,
        latest: { green: 300, lane_ms: 500, is_estimated: true },
        median: { green: 301, lane_ms: 502, is_estimated: false },
        constant_plays: 0,
        buckets: [],
    },
    notice: null,
    ai: null,
})

describe('판정 습관 화면', () => {
    test('녹색 숫자는 GREEN 값 뒤에 레인 통과 ms 를 붙이고, 오프셋은 노트가 보이는 위치를 옮긴다고 적는다', async () => {
        const installed = installFetchRoutes({ [`GET ${HABIT_PATH}`]: () => jsonResponse(habitView()) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiJudgeHabitPanel />)
        const greenLine = (await page.findByText(/최근 GREEN 300 \(500 ms\)/)).textContent ?? ''
        expect(greenLine).toContain('GREEN 300 (500 ms) 추정')
        expect(greenLine).toContain('GREEN 301 (502 ms)')
        expect(greenLine).not.toContain('GREEN 301 (502 ms) 추정')
        expect(page.getByText(/JUDGE OFFSET -4 ms/)).toBeTruthy()
        expect(page.getByText(/노트가 보이는 위치를 옮기고 판정은 옮기지 않습니다/)).toBeTruthy()
        expect(installed.calls.filter((call) => call.method === 'POST')).toHaveLength(0)
    })
})
