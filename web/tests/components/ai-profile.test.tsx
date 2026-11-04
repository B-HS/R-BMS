import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { SearchParamsContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/ai' })

afterAll(() => GlobalRegistrator.unregister())

const { changeValue, click, keyDown, page, submit, waitFor, within } = await import('../support/dom-harness')
const { AiProfilePanel } = await import('../../src/widgets/ai/ai-profile-panel')
const { AiHubTabs } = await import('../../src/widgets/ai/ai-hub-tabs')
const { AiCoachPanel } = await import('../../src/widgets/ai/ai-coach-panel')
const { AiGoalForm } = await import('../../src/features/ai/ai-goal-form')
const { RecordQueryFilterEditor } = await import('../../src/widgets/ai/record-query-filter-editor')
const { cleanupScreens, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')
const { weaknessProfileViewSchema } = await import('../../src/server/dto/ai-play')
const { aiGoalParamsSchema } = await import('../../src/server/dto/ai')

type Profile = import('../../src/entities/ai.type').AiWeaknessProfile
type ProfileCell = import('../../src/entities/ai.type').AiWeaknessProfileCell
type WeaknessItem = import('../../src/entities/ai.type').AiWeaknessItem
type CoachView = import('../../src/entities/ai.type').AiCoachView
type GoalInput = import('../../src/entities/ai.type').AiGoalInput
type RecordQueryPlan = import('../../src/entities/ai.type').AiRecordQueryPlan

const PROFILE_PATH = '/api/fe/me/ai/profile'
const PRACTICE_PATH = '/api/fe/me/ai/practice-folder'
const RECOMMEND_PATH = '/api/fe/me/ai/recommend'
const ROUTER = {
    push: () => undefined,
    replace: () => undefined,
    back: () => undefined,
    forward: () => undefined,
    refresh: () => undefined,
    prefetch: () => undefined,
}

let restoreFetch: () => void = () => undefined

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

const element = (selector: string) => {
    const found = document.querySelector(selector)
    if (!(found instanceof HTMLElement)) throw new Error(`missing ${selector}`)
    return found
}

const withRouter = (search: string, ui: React.ReactNode) => (
    <AppRouterContext.Provider value={ROUTER as never}>
        <SearchParamsContext.Provider value={new URLSearchParams(search)}>{ui}</SearchParamsContext.Provider>
    </AppRouterContext.Provider>
)

const cell = (overrides: Partial<ProfileCell> & Pick<ProfileCell, 'key' | 'kind'>): ProfileCell => ({
    value: -0.04,
    unit: 'ex_residual',
    ex_ratio: 0.8,
    bp_rate: 0.03,
    timing: null,
    notes: 900,
    charts: 12,
    plays: 20,
    auto_plays: 0,
    p_value: 0.01,
    q_threshold: 0.02,
    evidence: 'strong',
    leaning: 'weakness',
    sample_level: 'medium',
    trend_delta: null,
    interaction: null,
    ...overrides,
})

const weakness = (overrides: Partial<WeaknessItem> & Pick<WeaknessItem, 'key' | 'kind'>): WeaknessItem => ({
    label_tags: [],
    value: -0.06,
    unit: 'ex_residual',
    evidence: 'strong',
    sample_level: 'high',
    notes: 4200,
    charts: 25,
    plays: 60,
    severity: 0.06,
    trend_delta: 0.012,
    selectable_for_folder: true,
    training_kinds: ['trill'],
    ...overrides,
})

const curveBin = (from: number, exRatio: number, bpRate: number) => ({
    from_nps: from,
    to_nps: from + 2,
    notes: 400 + from * 10,
    charts: 6,
    ex_ratio: exRatio,
    ex_fit: exRatio,
    bp_rate: bpRate,
    bp_fit: bpRate,
})

const profileView = (overrides: Partial<Profile> = {}): Profile => ({
    profile_version: 1,
    window: { from: 0, to: 1, days: 90, plays: 120, charts: 40, notes: 98_000, stale: 0, auto_plays: 3, training_plays: 7 },
    density: {
        curve: [curveBin(4, 0.9, 0.01), curveBin(8, 0.88, 0.02), curveBin(12, 0.8, 0.05), curveBin(16, 0.72, 0.1)],
        baseline_ex: 0.89,
        comfort_nps: 10,
        limit_nps_ex: 16,
        limit_nps_bp: 16,
        max_seen_nps: 16,
        peak_seen_2s: 21.5,
        ceiling_reached_ex: true,
        evidence: 'strong',
        sustained_minus_burst: { value: -0.03, evidence: 'uncertain', sample_level: 'low' },
    },
    cells: [
        cell({ key: 'tag:trill', kind: 'tag', value: -0.06 }),
        cell({ key: 'combo:trill+stair', kind: 'combo', value: -0.08, interaction: -0.03 }),
        cell({ key: 'combo:chord_2+dense', kind: 'combo', value: 0.02, evidence: 'insufficient', leaning: null }),
        cell({
            key: 'hand:left',
            kind: 'hand',
            value: -0.02,
            timing: { mean_ms: 1.4, sd_ms: 12, fast_mean_ms: 9, slow_mean_ms: -8, fast_share: 0.6 },
        }),
        cell({ key: 'lane:S', kind: 'lane', value: -0.05 }),
        cell({
            key: 'timing:jack',
            kind: 'timing',
            value: -3.2,
            unit: 'ms',
            timing: { mean_ms: -1.8, sd_ms: 14, fast_mean_ms: 8, slow_mean_ms: -11, fast_share: 0.35 },
        }),
        cell({ key: 'bp_context:stair', kind: 'bp_context', value: 2.4, unit: 'ratio', notes: 80 }),
        cell({ key: 'gauge_context:dense', kind: 'gauge_context', value: 1.8, unit: 'ratio', notes: 60, evidence: 'uncertain', leaning: null }),
    ],
    weaknesses: [
        weakness({ key: 'tag:trill', kind: 'tag', label_tags: ['trill'] }),
        weakness({ key: 'lane:S', kind: 'lane', selectable_for_folder: false, training_kinds: [] }),
        weakness({ key: 'density:limit', kind: 'density', value: 16, unit: 'nps', training_kinds: ['stream'] }),
    ],
    strengths: [weakness({ key: 'tag:chord_2', kind: 'tag', value: 0.03, trend_delta: null })],
    bp_runs: { longest_mean: 6.5, histogram: { '1': 40, '2-3': 12 }, recovery_median: 18 },
    gauge: { drops_per_play: 1.2, drop_mean_nps: 15.4, overall_mean_nps: 9.8 },
    soflan: { events: 30, adjusted_share: 0.5 },
    notice: null,
    ...overrides,
})

const installProfile = (view: Profile) => {
    const installed = installFetchRoutes({ [`GET ${PROFILE_PATH}`]: () => jsonResponse(view) })
    restoreFetch = installed.restore
    return installed
}

describe('약점 프로필 화면', () => {
    test('시험 예시가 서버의 프로필 응답 스키마를 통과한다', () => {
        expect(weaknessProfileViewSchema.safeParse(profileView()).success).toBe(true)
    })

    test('분석된 판이 없으면 빈 상태를 보이고 AI 를 부르지 않는다', async () => {
        const installed = installProfile(
            profileView({
                window: { from: 0, to: 1, days: 90, plays: 0, charts: 0, notes: 0, stale: 0, auto_plays: 0, training_plays: 0 },
                cells: [],
                weaknesses: [],
                strengths: [],
                notice: { need: 'traced_plays' },
            }),
        )
        await renderWithQueryClient(<AiProfilePanel />)
        expect(await page.findByText('아직 분석된 판이 없습니다')).toBeTruthy()
        expect(page.queryByRole('img')).toBeNull()
        expect(installed.calls.every((call) => call.method === 'GET')).toBe(true)
    })

    test('표본이 모자라면 이유를 보이고 밀도 한계를 그리지 않으며 약점이 없다고 적는다', async () => {
        installProfile(
            profileView({
                window: { from: 0, to: 1, days: 90, plays: 6, charts: 4, notes: 3_000, stale: 2, auto_plays: 0, training_plays: 0 },
                density: { ...profileView().density, curve: [], comfort_nps: null, limit_nps_ex: null, limit_nps_bp: null, evidence: 'insufficient' },
                cells: [],
                weaknesses: [],
                strengths: [],
                notice: { need: 'traced_plays' },
            }),
        )
        await renderWithQueryClient(<AiProfilePanel />)
        expect(await page.findByText(/판별 데이터\(새 클라이언트로 친 판의 노트 단위 기록/)).toBeTruthy()
        expect(page.getByText('밀도 칸마다 노트와 차트가 모자라 밀도 한계를 그릴 수 없습니다.')).toBeTruthy()
        expect(page.getByText(/근거가 충분한 약점이 아직 없습니다/)).toBeTruthy()
        expect(page.getByText(/옛 분석 2판/)).toBeTruthy()
        expect(page.queryByRole('img')).toBeNull()
    })

    test('정상 프로필은 밀도 한계를 문장으로 단 SVG 와 약점·조합·손·문맥을 보이고, 약점마다 연습 폴더·훈련곡 링크를 단다', async () => {
        installProfile(profileView())
        await renderWithQueryClient(<AiProfilePanel />)

        const chart = await page.findByRole('img')
        expect(chart.tagName.toLowerCase()).toBe('svg')
        expect(chart.getAttribute('aria-label')).toBe(
            '밀도별 EX 비율 그래프. 편안한 밀도 10 NPS, EX 한계 16 NPS, BP 한계 16 NPS, 친 가장 높은 밀도 16 NPS.',
        )
        expect(chart.querySelectorAll('circle')).toHaveLength(4)
        expect(page.getByText(/훈련곡 판 7판/)).toBeTruthy()
        expect(page.getByText(/훈련곡 판은 곡이 달라도 한 묶음으로 셉니다/)).toBeTruthy()

        const weaknesses = element('ul[aria-label="약점"]')
        expect(within(weaknesses).getByText('트릴')).toBeTruthy()
        expect(weaknesses.textContent).toContain('-6.0%p')
        expect(weaknesses.textContent).toContain('최근 30일 +1.2%p')
        expect(within(weaknesses).getByRole('link', { name: '트릴 연습 폴더 만들기' }).getAttribute('href')).toBe(
            '/ai?tab=recommend&focus=tag%3Atrill',
        )
        expect(within(weaknesses).getByRole('link', { name: '트릴 훈련곡 만들기' }).getAttribute('href')).toBe('/ai?tab=training&focus=tag%3Atrill')
        expect(within(weaknesses).queryByRole('link', { name: '스크래치 레인 연습 폴더 만들기' })).toBeNull()
        expect(within(weaknesses).queryByRole('link', { name: '스크래치 레인 훈련곡 만들기' })).toBeNull()
        expect(weaknesses.textContent).toContain('16 NPS')
        expect(element('ul[aria-label="강점"]').textContent).toContain('2키 동시치기')

        const grid = element('table')
        expect(grid.querySelector('caption')?.textContent).toBe('패턴 조합별 EX 잔차')
        expect(grid.textContent).toContain('트릴 + 계단: -8.0%p, 근거 충분')
        expect(grid.textContent).not.toContain('2키 동시치기 + 고밀도:')

        expect(element('ul[aria-label="손별"]').textContent).toContain('평균 오차 +1.4 ms')
        expect(element('ul[aria-label="레인별"]').textContent).toContain('스크래치 레인')
        expect(element('ul[aria-label="패턴별 타이밍(내 평균 대비)"]').textContent).toContain('-3.2 ms')
        expect(page.getByRole('region', { name: 'BAD·POOR 연속 직전 패턴' }).textContent).toContain('×2.40')
        expect(page.getByRole('region', { name: '게이지 급락 중 패턴' }).textContent).toContain('×1.80')
    })

    test('밀도 한계에 닿지 않았으면 그래프 문장에 EX 한계 없음을 적고 더 높은 밀도를 쳐 보라고 한다', async () => {
        installProfile(
            profileView({
                density: { ...profileView().density, limit_nps_ex: null, limit_nps_bp: null, ceiling_reached_ex: false, evidence: 'uncertain' },
                weaknesses: [],
            }),
        )
        await renderWithQueryClient(<AiProfilePanel />)
        const chart = await page.findByRole('img')
        expect(chart.getAttribute('aria-label')).toContain('EX 한계 없음, BP 한계 없음')
        expect(page.getByText(/EX 가 한계만큼 떨어지지 않았습니다/)).toBeTruthy()
    })
})

describe('AI 허브의 약점 링크', () => {
    test('focus 가 있는 추천 탭 주소는 연습 폴더를 그 약점 출처로 열고 그 키로 후보를 묻는다', async () => {
        const searches: string[] = []
        const installed = installFetchRoutes({
            [`GET ${PROFILE_PATH}`]: () => jsonResponse(profileView()),
            [`GET ${RECOMMEND_PATH}`]: () => new Response(null, { status: 500 }),
            [`GET ${PRACTICE_PATH}`]: (_, query) => {
                searches.push(query.search)
                return jsonResponse({
                    source: 'weakness',
                    table_id: 'insane1',
                    tag: null,
                    goal_id: null,
                    focus_key: 'tag:trill',
                    candidates: [],
                    folders: { used: 1, limit: 50, can_create: true },
                    notice: { need: 'candidates' },
                    ai: null,
                })
            },
        })
        restoreFetch = installed.restore
        await renderWithQueryClient(withRouter('tab=recommend&focus=tag:trill', <AiHubTabs />))

        await waitFor(() => expect(searches.length).toBeGreaterThan(0))
        expect(new URLSearchParams(searches[0]).get('source')).toBe('weakness')
        expect(new URLSearchParams(searches[0]).get('focus_key')).toBe('tag:trill')
        expect((await page.findByLabelText('연습할 약점')).textContent).toBe('트릴')
        expect(page.getByRole('tab', { name: '추천·연습 폴더' }).getAttribute('aria-selected')).toBe('true')
        expect(page.queryByRole('tab', { name: '훈련곡' })).toBeNull()
    })

    test('profile 탭 주소는 약점 프로필 탭을 연다', async () => {
        installProfile(profileView())
        await renderWithQueryClient(withRouter('tab=profile', <AiHubTabs />))
        expect(page.getByRole('tab', { name: '약점 프로필' }).getAttribute('aria-selected')).toBe('true')
        expect(await page.findByRole('img')).toBeTruthy()
    })
})

describe('새 목표 종류 폼', () => {
    const renderForm = async (weaknessKeys: readonly string[]) => {
        const submitted: GoalInput[] = []
        await renderWithQueryClient(
            <AiGoalForm
                tableId='insane1'
                symbol='★'
                levels={['12']}
                weaknessKeys={weaknessKeys}
                isSubmitting={false}
                onSubmit={(input) => submitted.push(input)}
            />,
        )
        return submitted
    }

    const pickKind = async (name: string) => {
        await keyDown(page.getByLabelText('목표 종류'), 'ArrowDown')
        await click(await page.findByRole('option', { name }))
    }

    test('약점 잔차 목표는 프로필 약점과 %p 목표를 서버 입력 꼴로 보낸다', async () => {
        const submitted = await renderForm(['tag:trill', 'lane:S'])
        await pickKind('약점 잔차 끌어올리기')
        expect(page.getByLabelText('약점').textContent).toBe('트릴')
        await changeValue(page.getByLabelText('목표 잔차(%p, -20~5)') as HTMLInputElement, '-1.5')
        await submit(element('form[aria-label="새 목표"]') as HTMLFormElement)
        expect(submitted).toEqual([{ kind: 'weakness_residual', table: 'insane1', key: 'tag:trill', target_residual: -0.015 }])
        expect(aiGoalParamsSchema.safeParse(submitted[0]).success).toBe(true)
    })

    test('약점이 없으면 이유를 보이고 보내지 않으며, 밀도 목표는 범위 안의 NPS 만 보낸다', async () => {
        const submitted = await renderForm([])
        await pickKind('약점 잔차 끌어올리기')
        expect(page.getByText(/목표로 삼을 약점이 아직 없습니다/)).toBeTruthy()
        expect((page.getByRole('button', { name: '목표 추가' }) as HTMLButtonElement).disabled).toBe(true)

        await pickKind('밀도 한계 NPS 올리기')
        const nps = page.getByLabelText('목표 밀도 한계(NPS, 4~40)') as HTMLInputElement
        await changeValue(nps, '41')
        expect((page.getByRole('button', { name: '목표 추가' }) as HTMLButtonElement).disabled).toBe(true)
        await changeValue(nps, '14.5')
        await submit(element('form[aria-label="새 목표"]') as HTMLFormElement)
        expect(submitted).toEqual([{ kind: 'density_limit', table: 'insane1', target_nps: 14.5 }])
        expect(aiGoalParamsSchema.safeParse(submitted[0]).success).toBe(true)
    })
})

describe('기록 질문 조건의 새 항목', () => {
    test('훈련곡 항목은 참·거짓을 고르는 칸이고 패턴 태그 같음은 태그를 고르며, 중 하나는 글로 적는다', async () => {
        const runs: RecordQueryPlan[] = []
        await renderWithQueryClient(
            <RecordQueryFilterEditor
                initialPlan={{
                    filters: [
                        { field: 'training', op: 'eq', value: 'true' },
                        { field: 'tag', op: 'eq', value: 'trill' },
                        { field: 'tag', op: 'in', value: 'jack,stair' },
                    ],
                    sort_field: 'played_at',
                    sort_dir: 'desc',
                    limit: 20,
                }}
                issues={[]}
                isRunning={false}
                onRun={(plan) => runs.push(plan)}
            />,
        )
        expect(page.getByLabelText('조건 1 값').textContent).toBe('훈련곡만')
        expect(page.getByLabelText('조건 2 값').textContent).toBe('트릴')
        expect((page.getByLabelText('조건 3 값') as HTMLInputElement).value).toBe('jack,stair')

        await keyDown(page.getByLabelText('조건 1 값'), 'ArrowDown')
        await click(await page.findByRole('option', { name: '훈련곡 제외' }))
        await submit(element('form[aria-label="해석된 조건"]') as HTMLFormElement)
        expect(runs.at(-1)?.filters).toEqual([
            { field: 'training', op: 'eq', value: 'false' },
            { field: 'tag', op: 'eq', value: 'trill' },
            { field: 'tag', op: 'in', value: 'jack,stair' },
        ])
    })
})

describe('리플레이 코치의 실패 연속 표지', () => {
    test('BAD·POOR 연속이 시작된 구간 막대에 표지를 달고 연속과 겹치는 약점을 적는다', async () => {
        const segment = (index: number) => ({
            key: `seg.s${String(index).padStart(2, '0')}`,
            id: `s${String(index).padStart(2, '0')}`,
            from_ms: index * 1000,
            to_ms: (index + 1) * 1000,
            chart_notes: 10,
            notes: 10,
            ex_ratio: 0.7,
            bp: index,
            combo_breaks: 0,
            fast: 1,
            slow: 1,
            top_tags: [],
            gauge: 50,
            peak_nps: 12.5,
        })
        const view: CoachView = {
            play: {
                score_id: 'sc_run',
                played_at: 0,
                chart: { sha256: 'a'.repeat(64), md5: null, title: 'Chart', artist: 'Artist' },
                lamp: 'Failed',
                assist_grade: 0,
                ex_score: 10,
                max_ex_score: 60,
                minbp: 3,
                random: 'Off',
                gauge: 'Normal',
                scratch_auto: false,
            },
            analysis: {
                run_end: 'finished',
                overall: { notes: 30, ex_ratio: 0.7 },
                segments: [segment(0), segment(1), segment(2)],
                problem_segments: ['s02'],
                drops: [],
                gauge: { selected: 'normal', interval_ms: 500, curves: [] },
                pattern_join: 'absent',
                bp_runs: [
                    { from_ms: 1500, to_ms: 1900, length: 6, start_nps: 14, recovery_notes: null, preceding_tags: [{ tag: 'trill', notes: 5 }] },
                ],
                profile_keys: ['tag:trill'],
                drop_contexts: [],
            },
            notice: null,
            ai: null,
        }
        const installed = installFetchRoutes({ 'GET /api/fe/me/ai/plays/sc_run/coach': () => jsonResponse(view) })
        restoreFetch = installed.restore
        await renderWithQueryClient(<AiCoachPanel scoreId='sc_run' />)

        expect(await page.findByRole('button', { name: 's01 BP 1 · BAD·POOR 연속 시작' })).toBeTruthy()
        expect(page.getByRole('button', { name: 's00 BP 0' })).toBeTruthy()
        expect(element('ul[aria-label="BAD·POOR 연속"]').textContent).toContain('6노트 연속')
        expect(page.getByText('이 판의 문제 구간과 겹치는 내 약점: 트릴')).toBeTruthy()
        await click(page.getByRole('button', { name: 's02 BP 2' }))
        expect(page.getByRole('region', { name: '고른 구간' }).textContent).toContain('이 구간 최고 밀도 12.5 NPS')
    })
})
