import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { click, keyDown, page, runInAct, waitFor, within } = await import('../support/dom-harness')
const { AiReportPanel } = await import('../../src/widgets/ai/ai-report-panel')
const { AiCoachPanel } = await import('../../src/widgets/ai/ai-coach-panel')
const { AiComparePanel } = await import('../../src/widgets/ai/ai-compare-panel')
const { RivalSharingPanel } = await import('../../src/widgets/settings/rival-sharing-panel')
const { cleanupScreens, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')

type ReportView = import('../../src/entities/ai.type').AiReportView
type CoachView = import('../../src/entities/ai.type').AiCoachView
type CompareView = import('../../src/entities/ai.type').AiCompareView

const SCORE_ID = 'sc_contract'
const COACH_PATH = `/api/fe/me/ai/plays/${SCORE_ID}/coach`
const COMPARE_PATH = `/api/fe/me/ai/plays/${SCORE_ID}/compare`
const USAGE_PATH = '/api/fe/me/ai/usage'

let restoreFetch = () => {}

afterEach(async () => {
    restoreFetch()
    await cleanupScreens()
})

const reportView = (): ReportView => ({
    plays: { analyzed: 2, measured: 2, stale: 0, failed: 0, from: 0, to: 1 },
    tags: [
        { key: 'tag.trill', tag: 'trill', notes: 40, plays: 2, auto_plays: 0, ex_ratio: 0.5, residual: -0.12, grade: 'insufficient', leaning: null },
    ],
    lanes: [{ key: 'lane.1', name: '1', notes: 20, ex_ratio: 0.8, fast_share: 0.6 }],
    density: [],
    fatigue: { key: 'fatigue', plays: 2, mean_delta: -0.05 },
    timing: { key: 'timing.bias', notes: 30, mean_ms: 1.2 },
    options: [{ key: 'option.Off', option: 'Off', plays: 2, share: 1 }],
    preferred: [],
    lamps: null,
    score_rows: { plays: 2, fast: 10, slow: 5 },
    notice: { need: 'traced_plays' },
    ai: null,
})

const segment = (index: number, bp: number) => ({
    key: `seg.s${String(index).padStart(2, '0')}`,
    id: `s${String(index).padStart(2, '0')}`,
    from_ms: index * 400,
    to_ms: (index + 1) * 400,
    chart_notes: 2,
    notes: 2,
    ex_ratio: 0.75,
    bp,
    combo_breaks: bp,
    fast: 1,
    slow: 0,
    top_tags: [{ tag: 'trill', notes: 2 }],
    gauge: 50,
})

const coachView = (): CoachView => ({
    play: {
        score_id: SCORE_ID,
        played_at: 0,
        chart: { sha256: 'a'.repeat(64), md5: 'b'.repeat(32), title: '<b>Chart</b>', artist: 'Artist' },
        lamp: 'Normal',
        assist_grade: 0,
        ex_score: 41,
        max_ex_score: 64,
        minbp: 3,
        random: 'Off',
        gauge: 'Normal',
        scratch_auto: false,
    },
    analysis: {
        run_end: 'finished',
        overall: { notes: 64, ex_ratio: 0.64 },
        segments: Array.from({ length: 32 }, (_, index) => segment(index, index === 5 ? 3 : 0)),
        problem_segments: ['s05'],
        drops: [],
        gauge: { selected: 'normal', interval_ms: 500, curves: [{ gauge: 'hard', end: 0, cleared: null, outcome: 'unknown' }] },
        pattern_join: 'absent',
    },
    notice: null,
    ai: null,
})

describe('판별 기반 AI 화면', () => {
    test('리포트는 표본이 모자라면 결정론 숫자만 보이고 AI 버튼을 막는다', async () => {
        const installed = installFetchRoutes({ 'GET /api/fe/me/ai/report': () => jsonResponse(reportView()) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiReportPanel />)
        await waitFor(() => expect(document.querySelector('[aria-label="패턴별 잔차"]')?.textContent).toContain('트릴'))
        expect(page.getByText(/5판 아래/)).toBeTruthy()
        expect((page.getByRole('button', { name: 'AI 해석 만들기' }) as HTMLButtonElement).disabled).toBe(true)
        expect(installed.calls.filter((call) => call.method === 'POST')).toHaveLength(0)
    })

    test('코치는 구간 막대를 누르면 그 구간을 보이고, AI 는 버튼을 눌렀을 때 한 번만 부른다', async () => {
        const installed = installFetchRoutes({
            [`GET ${COACH_PATH}`]: () => jsonResponse(coachView()),
            [`POST ${COACH_PATH}`]: () =>
                jsonResponse({
                    output: {
                        summary: '6구간에서 무너졌습니다.',
                        segments: [{ segment_id: 's05', diagnosis: '트릴이 늦습니다.', advice: '손목을 푸세요.' }],
                        next_try: '천천히.',
                    },
                    cached: false,
                    is_current: true,
                    provider: 'anthropic',
                    model: 'claude-test',
                    usage: { inputTokens: 1, outputTokens: 1, cachedInputTokens: 0 },
                    created_at: 0,
                }),
            [`GET ${USAGE_PATH}`]: () => jsonResponse({}),
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiCoachPanel scoreId={SCORE_ID} />)
        expect(await page.findByText('<b>Chart</b>')).toBeTruthy()
        expect(installed.calls.filter((call) => call.method === 'POST')).toHaveLength(0)
        await click(page.getByRole('button', { name: 's05 BP 3' }))
        expect(page.getByRole('region', { name: '고른 구간' }).textContent).toContain('BP 3')
        await click(page.getByRole('button', { name: 'AI 해석 만들기' }))
        await waitFor(() => expect(page.getByRole('region', { name: 'AI 코치' }).textContent).toContain('트릴이 늦습니다'))
        expect(installed.calls.filter((call) => call.method === 'POST')).toHaveLength(1)
    })

    test('비교가 공개 판정 합계뿐이면 이유를 적고 AI 버튼을 두지 않는다', async () => {
        const totals = { lamp: 'Hard' as const, assist_grade: 0 as const, ex_score: 50, minbp: 1, pgreat: 20, great: 10, good: 0, bad: 0, poor: 1 }
        const view: CompareView = {
            against: 'best',
            base: { who: 'me', login_id: 'me', score_id: SCORE_ID, played_at: 0, lamp: 'Normal', ex_score: 41, random: 'Off', scratch_auto: false },
            target: {
                who: 'rival',
                login_id: 'rival',
                score_id: null,
                played_at: 0,
                lamp: 'Hard',
                ex_score: 50,
                random: 'Random',
                scratch_auto: false,
            },
            mode: 'totals',
            reason: 'rival_not_sharing',
            segments: [],
            gap_segments: [],
            totals: { base: { ...totals, ex_score: 41 }, target: totals },
            rivals: [{ login_id: 'rival', name: 'Rival', allows_segments: false }],
            notice: null,
            ai: null,
        }
        const installed = installFetchRoutes({ [`GET ${COMPARE_PATH}`]: () => jsonResponse(view) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiComparePanel scoreId={SCORE_ID} />)
        expect(await page.findByText(/공개된 판정 합계만 비교합니다/)).toBeTruthy()
        expect(page.queryByRole('button', { name: 'AI 해석 만들기' })).toBeNull()
    })

    const compareView = (overrides: Partial<CompareView> = {}): CompareView => ({
        against: 'best',
        base: { who: 'me', login_id: 'me', score_id: SCORE_ID, played_at: 0, lamp: 'Normal', ex_score: 41, random: 'Off', scratch_auto: false },
        target: { who: 'me', login_id: 'me', score_id: 'sc_best', played_at: 0, lamp: 'Hard', ex_score: 50, random: 'Off', scratch_auto: false },
        mode: 'segments',
        reason: null,
        segments: [
            { key: 'seg.s00', id: 's00', from_ms: 0, to_ms: 400, chart_notes: 2, ex_diff: -2, bp_diff: 1, cumulative_ex_diff: -2, top_tags: [] },
            {
                key: 'seg.s01',
                id: 's01',
                from_ms: 400,
                to_ms: 800,
                chart_notes: 2,
                ex_diff: null,
                bp_diff: null,
                cumulative_ex_diff: null,
                top_tags: [],
            },
            {
                key: 'seg.s02',
                id: 's02',
                from_ms: 800,
                to_ms: 1200,
                chart_notes: 2,
                ex_diff: null,
                bp_diff: null,
                cumulative_ex_diff: null,
                top_tags: [],
            },
        ],
        gap_segments: ['s00'],
        totals: null,
        rivals: [
            { login_id: 'rival', name: 'Rival', allows_segments: true },
            { login_id: 'quiet', name: 'Quiet', allows_segments: false },
        ],
        notice: null,
        ai: null,
        ...overrides,
    })

    test('판정 합계 표는 LIGHT ASSIST 판마다 끝난 게이지의 어시스트 등급을 따로 보인다', async () => {
        const judgements = { ex_score: 50, minbp: 1, pgreat: 20, great: 10, good: 0, bad: 0, poor: 1 }
        const view = compareView({
            mode: 'totals',
            reason: 'rival_not_sharing',
            segments: [],
            gap_segments: [],
            totals: {
                base: { ...judgements, lamp: 'LightAssistEasy', assist_grade: 4 },
                target: { ...judgements, lamp: 'LightAssistEasy', assist_grade: 0 },
            },
        })
        const installed = installFetchRoutes({ [`GET ${COMPARE_PATH}`]: () => jsonResponse(view) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiComparePanel scoreId={SCORE_ID} />)
        const baseRow = (await page.findByText('이 판')).closest('tr')
        const targetRow = page.getByText('비교 판').closest('tr')
        expect(baseRow?.textContent).toContain('ASSIST EXHARD')
        expect(targetRow?.textContent).toContain('LIGHT ASSIST')
        expect(targetRow?.textContent).not.toContain('ASSIST EXHARD')
    })

    test('한쪽 판이 도달하지 못한 구간은 차이를 보이지 않고 몇 구간을 뺐는지 알린다', async () => {
        const installed = installFetchRoutes({ [`GET ${COMPARE_PATH}`]: () => jsonResponse(compareView()) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiComparePanel scoreId={SCORE_ID} />)
        const rows = await page.findByLabelText('구간별 EX 차')
        expect(rows.textContent).toContain('s00')
        expect(rows.textContent).not.toContain('s01')
        expect(page.getByText(/도달하지 못한 2구간은 비교에서 뺐습니다/)).toBeTruthy()
    })

    test('도달하지 못한 구간이 있으면 머리의 차이도 도달한 구간만 더하고, 없으면 두 판의 EX 차이다', async () => {
        const partial = installFetchRoutes({ [`GET ${COMPARE_PATH}`]: () => jsonResponse(compareView()) })
        restoreFetch = partial.restore
        await renderWithQueryClient(<AiComparePanel scoreId={SCORE_ID} />)
        await page.findByLabelText('구간별 EX 차')
        const header = page.getByText(/이 판 EX 41/).textContent ?? ''
        expect(header).toContain('도달한 구간의 차이 -2')
        expect(header).not.toContain('-9')
        restoreFetch()
        await cleanupScreens()

        const whole = installFetchRoutes({
            [`GET ${COMPARE_PATH}`]: () => jsonResponse(compareView({ segments: compareView().segments.slice(0, 1), gap_segments: ['s00'] })),
        })
        restoreFetch = whole.restore
        await renderWithQueryClient(<AiComparePanel scoreId={SCORE_ID} />)
        await page.findByLabelText('구간별 EX 차')
        const wholeHeader = page.getByText(/이 판 EX 41/).textContent ?? ''
        expect(wholeHeader).toContain('차이 -9')
        expect(wholeHeader).not.toContain('도달한 구간')
    })

    test('비교 대상을 바꾸는 동안에도 라이벌 고르기 목록이 남고, 이전 대상의 비교는 보이지 않는다', async () => {
        let releaseRival = () => {}
        const rivalView = compareView({
            against: 'rival',
            target: { who: 'rival', login_id: 'rival', score_id: null, played_at: 0, lamp: 'Hard', ex_score: 60, random: 'Off', scratch_auto: false },
        })
        const installed = installFetchRoutes({
            [`GET ${COMPARE_PATH}`]: async (_call, { search }) => {
                if (!search.includes('rival=rival')) return jsonResponse(compareView())
                await new Promise<void>((resolve) => {
                    releaseRival = resolve
                })
                return jsonResponse(rivalView)
            },
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiComparePanel scoreId={SCORE_ID} />)
        await page.findByLabelText('구간별 EX 차')

        await keyDown(page.getByRole('combobox', { name: '비교할 판' }), 'ArrowDown')
        await click(await page.findByRole('option', { name: /Rival/ }))
        await waitFor(() => expect(page.getByRole('status', { name: '비교 대상 바꾸는 중' })).toBeTruthy())
        expect(page.queryByLabelText('구간별 EX 차')).toBeNull()

        await keyDown(page.getByRole('combobox', { name: '비교할 판' }), 'ArrowDown')
        const listbox = await page.findByRole('listbox')
        expect(
            within(listbox)
                .getAllByRole('option')
                .map((option) => option.textContent),
        ).toEqual(['내 다른 최고 판', 'Rival (구간 공유)', 'Quiet (합계만)'])

        await runInAct(() => releaseRival())
        await waitFor(() => expect(page.getByLabelText('구간별 EX 차')).toBeTruthy())
    })

    test('라이벌 구간 공유는 서로 라이벌인 플레이어에게만이라고 말한다', async () => {
        const installed = installFetchRoutes({ 'GET /api/fe/me/traces/sharing': () => jsonResponse({ rival_segments: false, updated_at: null }) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<RivalSharingPanel />)
        expect(await page.findByText(/서로 라이벌인 플레이어에게만/)).toBeTruthy()
    })

    test('라이벌 구간 공유는 기본 꺼짐이고, 켜면 rival_segments true 를 보낸다', async () => {
        const installed = installFetchRoutes({
            'GET /api/fe/me/traces/sharing': () => jsonResponse({ rival_segments: false, updated_at: null }),
            'PUT /api/fe/me/traces/sharing': () => jsonResponse({ rival_segments: true, updated_at: 1 }),
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(<RivalSharingPanel />)
        const toggle = await page.findByRole('switch')
        expect(toggle.getAttribute('aria-checked')).toBe('false')
        await click(toggle)
        await waitFor(() => expect(installed.calls.some((call) => call.method === 'PUT')).toBe(true))
        expect(installed.calls.find((call) => call.method === 'PUT')?.body).toEqual({ rival_segments: true })
    })
})
