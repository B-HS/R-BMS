import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

mock.module('@shared/lib/auth-client', () => ({
    authClient: { useSession: () => ({ data: null, isPending: false }), $store: { notify: () => undefined } },
}))

const { act, cleanup, fireEvent, render } = await import('@testing-library/react')
const fixture = await import('./member-dashboard-fixture')
const { withLocale } = await import('../support/browse-intl')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { formatDateTime } = await import('../../src/shared/lib/format')
const { dashboardCalendarQueryOptions, dashboardCoursesQueryOptions, dashboardOverviewQueryOptions, dashboardReplaysQueryOptions } =
    await import('../../src/entities/dashboard.query')
const { dashboardRivalsQueryOptions, dashboardTableProgressQueryOptions } = await import('../../src/entities/dashboard.query')
const { aiGaugeAdviceQueryOptions, aiGoalListQueryOptions, aiJudgeHabitQueryOptions, aiRecommendQueryOptions, aiWeeklyQueryOptions } =
    await import('../../src/entities/ai.query')
const { aiCredentialListQueryOptions, aiUsageSummaryQueryOptions } = await import('../../src/entities/ai.query')
const { folderListQueryOptions } = await import('../../src/entities/folder.query')
const { myDansQueryOptions } = await import('../../src/entities/dan.query')
const { activityRecentQueryOptions, statsSummaryQueryOptions } = await import('../../src/entities/stats.query')
const { MemberProfilePanel } = await import('../../src/widgets/home/member-profile-panel')
const { MemberSummaryPanel } = await import('../../src/widgets/home/member-summary-panel')
const { MemberLampPanel } = await import('../../src/widgets/home/member-lamp-panel')
const { MemberTableProgressPanel } = await import('../../src/widgets/home/member-table-progress-panel')
const { MemberCalendarPanel } = await import('../../src/widgets/home/member-calendar-panel')
const { MemberRecentPanel } = await import('../../src/widgets/home/member-recent-panel')
const { MemberDanCoursePanel } = await import('../../src/widgets/home/member-dan-course-panel')
const { MemberWeekPanel } = await import('../../src/widgets/home/member-week-panel')
const { MemberGoalPanel } = await import('../../src/widgets/home/member-goal-panel')
const { MemberRivalPanel } = await import('../../src/widgets/home/member-rival-panel')
const { MemberFolderReplayPanel } = await import('../../src/widgets/home/member-folder-replay-panel')
const { MemberAiStatusPanel } = await import('../../src/widgets/home/member-ai-status-panel')
const { MemberCommunityPanel } = await import('../../src/widgets/home/member-community-panel')

const { LOGIN_ID } = fixture

type AiWeeklyView = import('../../src/entities/ai.type').AiWeeklyView
type AiRecommendView = import('../../src/entities/ai.type').AiRecommendView
type AiGoal = import('../../src/entities/ai.type').AiGoal
type AiJudgeHabitView = import('../../src/entities/ai.type').AiJudgeHabitView
type AiGaugeAdviceView = import('../../src/entities/ai.type').AiGaugeAdviceView
type MyDans = import('../../src/entities/dan.type').MyDans
type FolderSummary = import('../../src/entities/folder.type').FolderSummary

const RECENT_PARAMS = { page: 1, limit: 20 }
const COMMUNITY_PARAMS = { limit: 10 }
const SHA = 'a'.repeat(64)
const DAY_MS = 86_400_000
const LOAD_ERROR_SENTENCE = '내 정보를 불러오지 못했습니다.'

let network: ReturnType<typeof fixture.stubFetch>
let observer: ReturnType<typeof fixture.installIntersectionObserver>

beforeEach(() => {
    network = fixture.stubFetch(() => 'pending')
    observer = fixture.installIntersectionObserver()
})

afterEach(() => {
    cleanup()
    network.restore()
    observer.restore()
})

const enterView = () => act(() => observer.enter())

const SETTLE_ROUNDS = 3

/** Lets the stubbed requests answer and React Query hand the answers to the screen; the retrying finders of Testing Library keep timers of the first DOM a test file registered. */
const settle = async () => {
    for (let round = 0; round < SETTLE_ROUNDS; round += 1) await act(async () => void (await new Promise<void>((resolve) => setTimeout(resolve, 0))))
}

const renderPanel = (ui: ReactNode, seed: (queryClient: QueryClient) => void = () => undefined) => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false } } })
    seed(queryClient)
    return render(<QueryClientProvider client={queryClient}>{withLocale(ui)}</QueryClientProvider>)
}

const weeklyView = (): AiWeeklyView => {
    const totals = {
        plays: 21,
        charts: 9,
        active_days: 4,
        first_plays: 1,
        lamp_ups: 3,
        new_clears: 2,
        ex_ups: 5,
        fast: 10,
        slow: 8,
        fast_share: 0.55,
        mean_offset_ms: 1.5,
    }
    return {
        week: { start: '2026-11-02', end: '2026-11-08', in_progress: true },
        totals,
        previous: { ...totals, plays: 15, active_days: 5, lamp_ups: 3, new_clears: 0 },
        lamp_changes: [],
        ex_changes: [],
        levels: [],
        notice: null,
        ai: null,
    }
}

const recommendView = (): AiRecommendView => {
    const candidate = (key: string, kind: 'failed' | 'unplayed' | 'lamp_up') => ({
        key,
        md5: key.padEnd(32, 'c'),
        title: `Challenge ${key}`,
        artist: 'artist',
        level: '12',
        lamp: 'Failed' as const,
        assist_grade: 0 as const,
        ex_rate: 0.7,
        bp: 30,
        tags: [],
        kind,
        score: 1,
    })
    return {
        table: { id: 'insane1', name: '発狂BMS難易度表', symbol: '★' },
        frontier: null,
        levels: [],
        candidates: [candidate('a', 'failed'), candidate('b', 'unplayed'), candidate('c', 'lamp_up'), candidate('d', 'failed')],
        pattern_coverage: { with_features: 4, candidates: 4, stale_features: 0 },
        notice: null,
        ai: null,
    }
}

const goal = (id: string, achieved: number, done: boolean): AiGoal => ({
    id,
    params: { kind: 'clear_all_level', table: 'insane1', level: '12' },
    created_at: fixture.LAST_PLAYED_AT,
    progress: { label: `Goal ${id}`, target: 50, achieved, done, remaining: 50 - achieved, recent_updates: 2 },
})

const judgeHabitView = (overrides: Partial<AiJudgeHabitView> = {}): AiJudgeHabitView => ({
    plays: 40,
    from: null,
    to: null,
    fast_slow: { fast: 55, slow: 45, fast_share: 0.55, grade: 'strong', leaning: 'fast', by_grade: [] },
    offset: {
        plays: 40,
        notes: 9000,
        mean_offset_ms: -4.25,
        grade: 'strong',
        direction: 'later',
        suggested_shift_ms: 4,
        auto_adjust_plays: 0,
        source: 'trace',
    },
    trace_timing: { plays: 40, notes: 9000, mean_ms: null, ci_low_ms: null, ci_high_ms: null, grade: 'strong', lanes: [] },
    avgjudge: { plays: 0, older_ms: null, newer_ms: null, change_ms: null, grade: 'insufficient' },
    green: { plays: 0, latest: null, median: null, constant_plays: 0, buckets: [] },
    notice: null,
    ai: null,
    ...overrides,
})

const gaugeAdviceView = (overrides: Partial<AiGaugeAdviceView> = {}): AiGaugeAdviceView => ({
    plays: { analyzed: 30, stale: 0, failed: 9, unknown_runs: 0 },
    gauges: [],
    rescues: [
        {
            key: 'r1',
            level: '12',
            played: 'hard',
            failed: 5,
            rescued_by: [
                { gauge: 'normal', plays: 3 },
                { gauge: 'easy', plays: 1 },
            ],
        },
        { key: 'r2', level: '11', played: 'hard', failed: 4, rescued_by: [{ gauge: 'easy', plays: 3 }] },
    ],
    options: [],
    score_rows: [],
    notice: null,
    ai: null,
    ...overrides,
})

const myDans = (): MyDans => ({
    systems: [
        {
            system: 'stella',
            name: 'Stella',
            display: { mode: 'auto', label: null, assisted: false },
            shown: [{ system: 'stella', name: 'Stella', label: 'st3', assisted: false }],
            acquired: [
                {
                    label: 'st3',
                    assisted: false,
                    rank: 3,
                    course_hash: 'c'.repeat(64),
                    course_name: 'Stella st3',
                    course_gauge: 'LR2',
                    acquired_at: fixture.LAST_PLAYED_AT,
                },
            ],
        },
        {
            system: 'satellite',
            name: 'Satellite',
            display: { mode: 'auto', label: null, assisted: false },
            shown: [],
            acquired: [],
        },
    ],
})

const folder = (index: number): FolderSummary => ({
    id: `uf_${String(index).padStart(30, '0')}`,
    name: `Folder ${index}`,
    description: '',
    is_public: false,
    position: index,
    item_count: index * 10,
    updated_at: fixture.LAST_PLAYED_AT + index * DAY_MS,
})

describe('프로필 패널', () => {
    test('이름·아이디·대표 단위·종합 순위·마지막 플레이와 링크를 보인다', () => {
        const view = renderPanel(<MemberProfilePanel loginId={LOGIN_ID} />, (queryClient) => {
            queryClient.setQueryData(QUERY_KEY.PLAYER.PROFILE(LOGIN_ID), fixture.profile())
            queryClient.setQueryData(dashboardOverviewQueryOptions().queryKey, fixture.overview())
        })
        expect(view.getByRole('heading', { name: '내 정보' })).toBeTruthy()
        expect(view.getByText('Alice')).toBeTruthy()
        expect(view.getByText('@alice')).toBeTruthy()
        expect(view.getByText('Stella st3')).toBeTruthy()
        expect(view.getByText('3위 / 120명')).toBeTruthy()
        expect(view.getByText(formatDateTime(fixture.LAST_PLAYED_AT))).toBeTruthy()
        expect(view.getByRole('link', { name: '내 페이지' }).getAttribute('href')).toBe('/players/alice')
        expect(view.getByRole('link', { name: '설정' }).getAttribute('href')).toBe('/settings')
    })

    test('순위도 판도 없는 사람은 순위 없음과 기록 없음을 보인다', () => {
        const view = renderPanel(<MemberProfilePanel loginId={LOGIN_ID} />, (queryClient) => {
            queryClient.setQueryData(QUERY_KEY.PLAYER.PROFILE(LOGIN_ID), fixture.profile())
            queryClient.setQueryData(dashboardOverviewQueryOptions().queryKey, fixture.emptyOverview())
        })
        expect(view.getByText('순위 없음')).toBeTruthy()
        expect(view.getByText('기록 없음')).toBeTruthy()
    })

    test('불러오는 동안 스켈레톤을 보이고 두 요청을 한 번씩 보낸다', () => {
        const view = renderPanel(<MemberProfilePanel loginId={LOGIN_ID} />)
        expect(view.getByRole('status', { name: '내 정보 불러오는 중' })).toBeTruthy()
        expect(network.calls.toSorted()).toEqual(['/api/fe/me/dashboard/overview', '/api/players/alice'])
    })

    test('실패하면 문구와 다시 시도 단추를 보이고 단추로 다시 요청한다', async () => {
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const view = renderPanel(<MemberProfilePanel loginId={LOGIN_ID} />)
        await settle()
        expect(view.getByRole('alert').textContent).toBe(LOAD_ERROR_SENTENCE)
        const callsBefore = network.calls.length
        fireEvent.click(view.getByRole('button', { name: '다시 시도' }))
        await settle()
        expect(network.calls.length).toBeGreaterThan(callsBefore)
    })
})

describe('플레이 요약 패널', () => {
    test('타일 여덟 개에 값과 보조 문구를 보인다', () => {
        const view = renderPanel(<MemberSummaryPanel />, (queryClient) => {
            queryClient.setQueryData(dashboardOverviewQueryOptions().queryKey, fixture.overview())
        })
        const tiles = view.getAllByRole('term').map((term) => term.textContent)
        expect(tiles).toEqual([
            '총 판',
            '플레이한 차트',
            '클리어 차트',
            'HARD 이상',
            'FULL COMBO 이상',
            '30일 플레이 시간',
            '연속 플레이',
            '이번 주 판',
        ])
        expect(view.getByText('1,234')).toBeTruthy()
        expect(view.getByText('75.00%', { exact: false })).toBeTruthy()
        expect(view.getByText('2:00')).toBeTruthy()
        expect(view.getByText('활동 12일')).toBeTruthy()
        expect(view.getByText('3일')).toBeTruthy()
        expect(view.getByText('최근 1년 최고 9일')).toBeTruthy()
        expect(view.getByText('21')).toBeTruthy()
    })

    test('제출된 판이 없으면 안내와 연결 방법 링크를 보인다', () => {
        const view = renderPanel(<MemberSummaryPanel />, (queryClient) => {
            queryClient.setQueryData(dashboardOverviewQueryOptions().queryKey, fixture.emptyOverview())
        })
        expect(view.getByText('아직 제출된 판이 없습니다. 클라이언트로 플레이하면 여기에 쌓입니다.')).toBeTruthy()
        expect(view.getByRole('link', { name: '연결 방법 보기' }).getAttribute('href')).toBe('/guide')
        expect(view.queryAllByRole('term')).toHaveLength(0)
    })

    test('불러오는 중과 실패를 가른다', async () => {
        const loading = renderPanel(<MemberSummaryPanel />)
        expect(loading.getByRole('status', { name: '플레이 요약 불러오는 중' })).toBeTruthy()
        cleanup()
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const failed = renderPanel(<MemberSummaryPanel />)
        await settle()
        expect(failed.getByRole('alert').textContent).toBe('플레이 요약을 불러오지 못했습니다.')
    })
})

describe('램프 패널', () => {
    const seed = (queryClient: QueryClient) => {
        queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(LOGIN_ID), fixture.stats())
        queryClient.setQueryData(dashboardOverviewQueryOptions().queryKey, fixture.overview())
    }

    test('램프 분포와 SCRATCH AUTO 안내와 DJ 등급 막대 아홉 개를 보인다', () => {
        const view = renderPanel(<MemberLampPanel loginId={LOGIN_ID} />, seed)
        expect(view.getByText('클리어한 차트')).toBeTruthy()
        expect(view.getByText('SCRATCH AUTO 기록 7곡 포함')).toBeTruthy()
        const grades = view.getByRole('list', { name: 'DJ 등급별 차트 수' })
        expect([...grades.querySelectorAll('li')].map((item) => item.textContent)).toEqual([
            'F1',
            'E2',
            'D3',
            'C4',
            'B5',
            'A6',
            'AA7',
            'AAA8',
            'MAX9',
        ])
        expect(view.getByRole('img', { name: 'MAX 9' })).toBeTruthy()
        expect(view.getByText('차트마다 최고 기록 기준입니다. 노트 수를 모르는 차트는 제외합니다.')).toBeTruthy()
    })

    test('플레이한 차트가 없으면 빈 상태를 보인다', () => {
        const view = renderPanel(<MemberLampPanel loginId={LOGIN_ID} />, (queryClient) => {
            queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(LOGIN_ID), { total_plays: 0, cleared: 0, lamps: [], scratch_auto_charts: 0 })
        })
        expect(view.getByText('아직 플레이한 차트가 없습니다.')).toBeTruthy()
    })

    test('등급 분포만 늦어도 램프 분포는 먼저 그리고 등급 자리에 스켈레톤을 둔다', () => {
        const view = renderPanel(<MemberLampPanel loginId={LOGIN_ID} />, (queryClient) => {
            queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(LOGIN_ID), fixture.stats())
        })
        expect(view.getByText('클리어한 차트')).toBeTruthy()
        expect(view.getByRole('status', { name: 'DJ 등급 분포 불러오는 중' })).toBeTruthy()
    })
})

describe('표별 진행 패널', () => {
    test('첫 표는 곧바로 요청하고 레벨마다 한 줄을 보인다', () => {
        const view = renderPanel(<MemberTableProgressPanel />, (queryClient) => {
            queryClient.setQueryData(dashboardTableProgressQueryOptions('insane1').queryKey, fixture.tableProgress())
        })
        const rows = view.getByRole('list', { name: '레벨별 램프 분포' })
        expect([...rows.querySelectorAll('li')].map((row) => row.textContent)).toEqual(['★14/43', '★22/62'])
        expect(view.getByText('6', { selector: 'span.font-medium' })).toBeTruthy()
        expect(view.getByRole('radio', { name: '★ 発狂BMS' }).getAttribute('aria-checked')).toBe('true')
    })

    test('불러오는 표는 첫 표뿐이고 다른 표는 고를 때 요청한다', () => {
        const view = renderPanel(<MemberTableProgressPanel />)
        expect(network.calls).toEqual(['/api/fe/me/dashboard/tables/insane1/progress'])
        fireEvent.click(view.getByRole('radio', { name: 'Stella' }))
        expect(view.getByRole('radio', { name: 'Stella' }).getAttribute('aria-checked')).toBe('true')
        expect(network.calls).toEqual(['/api/fe/me/dashboard/tables/insane1/progress', '/api/fe/me/dashboard/tables/stella-st/progress'])
        expect(view.getByRole('status', { name: '표별 진행 불러오는 중' })).toBeTruthy()
    })

    test('표를 불러오지 못하면 문구를 보인다', async () => {
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const view = renderPanel(<MemberTableProgressPanel />)
        await settle()
        expect(view.getByRole('alert').textContent).toBe('표별 진행을 불러오지 못했습니다.')
    })
})

describe('활동 패널', () => {
    const seed = (queryClient: QueryClient) => {
        queryClient.setQueryData(dashboardCalendarQueryOptions(182).queryKey, fixture.calendar())
        queryClient.setQueryData(QUERY_KEY.PLAYER.PLAY_DAYS(LOGIN_ID), fixture.playDays())
    }

    test('182일 달력은 월요일을 맨 위로 첫날을 요일 칸에 맞추고 판 수에 따라 다섯 단계로 칠한다', () => {
        const view = renderPanel(<MemberCalendarPanel loginId={LOGIN_ID} />, seed)
        const grid = view.getByRole('img', { name: '최근 182일 플레이 달력, 활동 166일, 991판' })
        const cells = [...grid.children] as HTMLElement[]
        expect(cells).toHaveLength(182)
        expect(cells[0].getAttribute('title')).toBe('2026-05-06 · 0판')
        expect(cells[0].style.gridRowStart).toBe('3')
        const levelOf = (plays: number) => cells[plays].className.split(' ').find((name) => name.startsWith('bg-')) ?? ''
        expect([0, 1, 2, 3, 5, 6, 9, 10, 11].map(levelOf)).toEqual([
            'bg-muted',
            'bg-primary/25',
            'bg-primary/25',
            'bg-primary/45',
            'bg-primary/45',
            'bg-primary/70',
            'bg-primary/70',
            'bg-primary',
            'bg-primary',
        ])
        expect(view.getByText('2026-05-06 – 2026-11-03')).toBeTruthy()
    })

    test('연속 일·기간 내 최고·활동 일 수와 30일 플레이 시간 막대를 보인다', () => {
        const view = renderPanel(<MemberCalendarPanel loginId={LOGIN_ID} />, seed)
        expect(view.getByText('4일')).toBeTruthy()
        expect(view.getByText('11일')).toBeTruthy()
        expect(view.getByText('166일')).toBeTruthy()
        const chart = view.getByRole('list', { name: '최근 30일 일별 플레이 시간 그래프' })
        expect(chart.querySelectorAll('[role="img"]')).toHaveLength(30)
        expect(view.getByText('최근 30일 합계 1:05')).toBeTruthy()
    })

    test('달력과 30일 막대가 각자 빈 상태·로딩을 따로 가진다', () => {
        const view = renderPanel(<MemberCalendarPanel loginId={LOGIN_ID} />, (queryClient) => {
            queryClient.setQueryData(QUERY_KEY.PLAYER.PLAY_DAYS(LOGIN_ID), { days: fixture.playDays().days, total_plays: 0, total_ms: 0 })
        })
        expect(view.getByRole('status', { name: '플레이 달력 불러오는 중' })).toBeTruthy()
        expect(view.getByText('최근 플레이한 날이 없습니다.')).toBeTruthy()
    })

    test('달력을 불러오지 못하면 그 자리에만 오류를 보인다', async () => {
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const view = renderPanel(<MemberCalendarPanel loginId={LOGIN_ID} />, (queryClient) => {
            queryClient.setQueryData(QUERY_KEY.PLAYER.PLAY_DAYS(LOGIN_ID), fixture.playDays())
        })
        await settle()
        expect(view.getByRole('alert').textContent).toBe('플레이 달력을 불러오지 못했습니다.')
        expect(view.getByText('최근 30일 합계 1:05')).toBeTruthy()
    })
})

describe('최근 플레이 패널', () => {
    const seed = (rows: ReturnType<typeof fixture.recentRow>[]) => (queryClient: QueryClient) => {
        queryClient.setQueryData(QUERY_KEY.PLAYER.RECENT(LOGIN_ID, RECENT_PARAMS), {
            data: rows,
            pagination: { page: 1, limit: 20, total: rows.length, totalPages: 1 },
        })
    }

    test('판마다 한 줄에 A-SCR 표지와 판 코치 링크를 달고 전체 보기 링크를 둔다', () => {
        const rows = [fixture.recentRow('Normal chart'), fixture.recentRow('Scratch chart', { scratch_auto: true })]
        const view = renderPanel(<MemberRecentPanel loginId={LOGIN_ID} />, seed(rows))
        const items = view.getAllByRole('listitem')
        expect(items).toHaveLength(2)
        expect(items[0].textContent).not.toContain('A-SCR')
        expect(items[1].textContent).toContain('A-SCR')
        expect(view.getAllByRole('link', { name: '판 코치' })[0].getAttribute('href')).toBe('/ai/plays/Normal%20chart')
        expect(view.getByRole('link', { name: '전체 보기' }).getAttribute('href')).toBe('/players/alice')
    })

    test('훈련곡 목록에 든 차트에만 TRAINING 표지를 단다', () => {
        const rows = [fixture.recentRow('Trained', { chart: { md5: 'b'.repeat(32), sha256: SHA, title: 'Trained', artist: 'artist', level: 12 } })]
        const withSet = renderPanel(<MemberRecentPanel loginId={LOGIN_ID} trainingSha256s={new Set([SHA])} />, seed(rows))
        expect(withSet.getByText('TRAINING · artist')).toBeTruthy()
        cleanup()
        const without = renderPanel(<MemberRecentPanel loginId={LOGIN_ID} />, seed(rows))
        expect(without.queryByText('TRAINING · artist')).toBeNull()
        expect(without.getByText('artist')).toBeTruthy()
    })

    test('빈 목록과 불러오는 중과 실패를 가른다', async () => {
        const empty = renderPanel(<MemberRecentPanel loginId={LOGIN_ID} />, seed([]))
        expect(empty.getByText('아직 플레이한 기록이 없습니다.')).toBeTruthy()
        cleanup()
        const loading = renderPanel(<MemberRecentPanel loginId={LOGIN_ID} />)
        expect(loading.getByRole('status', { name: '최근 플레이 불러오는 중' })).toBeTruthy()
        cleanup()
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const failed = renderPanel(<MemberRecentPanel loginId={LOGIN_ID} />)
        await settle()
        expect(failed.getByRole('alert').textContent).toBe('최근 플레이를 불러오지 못했습니다.')
    })
})

describe('단위·코스 패널', () => {
    test('체계별 표시 단위와 코스 묶음별 클리어와 최근 코스 기록을 보인다', () => {
        const view = renderPanel(<MemberDanCoursePanel />, (queryClient) => {
            queryClient.setQueryData(myDansQueryOptions().queryKey, myDans())
            queryClient.setQueryData(dashboardCoursesQueryOptions().queryKey, fixture.courses())
        })
        const systems = view.getByRole('list', { name: '체계별 표시 단위' })
        expect([...systems.querySelectorAll(':scope > li')].map((item) => item.textContent)).toEqual([
            'Stella st3취득 1',
            'Satellite · 취득한 단위 없음취득 0',
        ])
        expect(view.getByRole('img', { name: 'Stella: 클리어 4, 플레이 6, 전체 10' })).toBeTruthy()
        expect(view.getByText('4/10')).toBeTruthy()
        expect(view.getByRole('link', { name: 'Stella st3' }).getAttribute('href')).toBe(`/courses/${'c'.repeat(64)}`)
        expect(view.getByText('2,345')).toBeTruthy()
    })

    test('코스를 한 번도 치지 않았으면 코스 구역에 빈 상태를 보인다', () => {
        const view = renderPanel(<MemberDanCoursePanel />, (queryClient) => {
            queryClient.setQueryData(myDansQueryOptions().queryKey, myDans())
            queryClient.setQueryData(dashboardCoursesQueryOptions().queryKey, {
                groups: [{ id: 'stella', group: 'Stella', total: 10, played: 0, cleared: 0 }],
                recent: [],
            })
        })
        expect(view.getByText('아직 플레이한 코스가 없습니다.')).toBeTruthy()
    })

    test('단위와 코스가 따로 불러오고 따로 실패한다', async () => {
        network.restore()
        network = fixture.stubFetch((url) => (url.endsWith('/dashboard/courses') ? fixture.failure() : 'pending'))
        const view = renderPanel(<MemberDanCoursePanel />)
        expect(view.getByRole('status', { name: '단위 불러오는 중' })).toBeTruthy()
        await settle()
        expect(view.getByRole('alert').textContent).toBe('코스 진행을 불러오지 못했습니다.')
    })
})

describe('이번 주 패널', () => {
    test('화면에 들어오기 전에는 요청하지 않고 들어오면 주간 요약과 추천을 요청한다', async () => {
        network.restore()
        network = fixture.stubFetch((url) => {
            if (url.includes('/ai/weekly')) return fixture.envelope(weeklyView())
            if (url.includes('/ai/recommend')) return fixture.envelope(recommendView())
            return 'pending'
        })
        const view = renderPanel(<MemberWeekPanel />)
        expect(network.calls).toEqual([])
        expect(view.getAllByRole('status')).toHaveLength(2)
        enterView()
        await settle()
        expect(view.getByText('Challenge a', { exact: false })).toBeTruthy()
        expect(network.calls.toSorted()).toEqual(['/api/fe/me/ai/recommend', '/api/fe/me/ai/weekly'])
    })

    test('이번 주 숫자를 앞 주와 견주고 다음 도전 곡은 셋만 보인다', () => {
        const view = renderPanel(<MemberWeekPanel />, (queryClient) => {
            queryClient.setQueryData(aiWeeklyQueryOptions(null).queryKey, weeklyView())
            queryClient.setQueryData(aiRecommendQueryOptions(null).queryKey, recommendView())
        })
        expect(view.getByText('진행 중')).toBeTruthy()
        expect(view.getByText('2026-11-02 – 2026-11-08')).toBeTruthy()
        expect(view.getByText('앞 주 대비 +6')).toBeTruthy()
        expect(view.getByText('앞 주 대비 -1')).toBeTruthy()
        expect(view.getAllByText('앞 주 대비 0')).toHaveLength(1)
        expect(view.getByText('앞 주 대비 +2')).toBeTruthy()
        const challenges = view.getByRole('list', { name: '다음 도전 곡 목록' })
        expect([...challenges.querySelectorAll('li')].map((item) => item.textContent)).toEqual([
            '★12 Challenge a실패한 곡FAILED',
            '★12 Challenge b안 친 곡FAILED',
            '★12 Challenge c램프 올리기FAILED',
        ])
        expect(view.getByRole('link', { name: '주간 요약' }).getAttribute('href')).toBe('/ai?tab=weekly')
    })

    test('추천할 곡이 없으면 안내를 보이고 데이터가 모자라면 그 이유를 보인다', () => {
        const view = renderPanel(<MemberWeekPanel />, (queryClient) => {
            queryClient.setQueryData(aiWeeklyQueryOptions(null).queryKey, { ...weeklyView(), notice: { need: 'week_plays' } })
            queryClient.setQueryData(aiRecommendQueryOptions(null).queryKey, { ...recommendView(), candidates: [] })
        })
        expect(view.getByText('추천할 곡이 아직 없습니다.')).toBeTruthy()
        expect(view.getByRole('status').textContent).toContain('이 주에 친 판이 없습니다')
    })
})

describe('목표 패널', () => {
    test('화면에 들어오기 전에는 요청하지 않는다', () => {
        renderPanel(<MemberGoalPanel />)
        expect(network.calls).toEqual([])
        enterView()
        expect(network.calls).toEqual(['/api/fe/me/ai/goals'])
    })

    test('활성 목표를 진행 막대와 함께 다섯 개까지 보이고 나머지는 개수로 말한다', () => {
        const goals = Array.from({ length: 7 }, (_, index) => goal(`g${index}`, index === 0 ? 50 : 20, index === 0))
        const view = renderPanel(<MemberGoalPanel />, (queryClient) => {
            queryClient.setQueryData(aiGoalListQueryOptions().queryKey, goals)
        })
        const items = view.getByRole('list', { name: '활성 목표' }).querySelectorAll(':scope > li')
        expect(items).toHaveLength(5)
        expect(items[0].textContent).toContain('달성')
        expect(items[1].textContent).toContain('20 / 50')
        expect(view.getByRole('progressbar', { name: 'Goal g1 진행률' }).getAttribute('aria-valuenow')).toBe('40')
        expect(view.getByText('그 밖에 2개')).toBeTruthy()
    })

    test('목표가 없으면 만들기 링크를 보인다', () => {
        const view = renderPanel(<MemberGoalPanel />, (queryClient) => {
            queryClient.setQueryData(aiGoalListQueryOptions().queryKey, [])
        })
        expect(view.getByText('활성 목표가 없습니다.')).toBeTruthy()
        expect(view.getByRole('link', { name: '목표 만들기' }).getAttribute('href')).toBe('/ai?tab=goals')
    })
})

describe('라이벌 패널', () => {
    test('화면에 들어오기 전에는 요청하지 않고 들어오면 열 개까지 요청한다', () => {
        renderPanel(<MemberRivalPanel />)
        expect(network.calls).toEqual([])
        enterView()
        expect(network.calls).toEqual(['/api/fe/me/dashboard/rivals?limit=10'])
    })

    test('라이벌별 승-무-패와 공통 곡 없음과 라이벌의 최근 플레이를 보인다', () => {
        const view = renderPanel(<MemberRivalPanel />, (queryClient) => {
            queryClient.setQueryData(dashboardRivalsQueryOptions(10).queryKey, fixture.rivals())
        })
        expect(view.getByText('6-1-3')).toBeTruthy()
        expect(view.getByRole('img', { name: 'Bob: 공통 10곡 중 6승 1무 3패' })).toBeTruthy()
        expect(view.getByText('공통 곡 없음')).toBeTruthy()
        expect(view.getByRole('link', { name: 'Carol' }).getAttribute('href')).toBe('/players/carol')
        const recent = view.getByRole('list', { name: '라이벌 최근 플레이 목록' })
        expect(recent.querySelectorAll('li')).toHaveLength(1)
    })

    test('라이벌이 없으면 등록 링크를 보인다', () => {
        const view = renderPanel(<MemberRivalPanel />, (queryClient) => {
            queryClient.setQueryData(dashboardRivalsQueryOptions(10).queryKey, { rivals: [], recent: [], versus: [] })
        })
        expect(view.getByText('등록한 라이벌이 없습니다.')).toBeTruthy()
        expect(view.getByRole('link', { name: '라이벌 등록' }).getAttribute('href')).toBe('/settings?tab=rivals')
    })
})

describe('폴더·리플레이 패널', () => {
    test('최근 수정한 폴더 다섯 개와 최근 리플레이를 링크로 보인다', () => {
        const view = renderPanel(<MemberFolderReplayPanel />, (queryClient) => {
            queryClient.setQueryData(folderListQueryOptions().queryKey, [1, 2, 3, 4, 5, 6, 7].map(folder))
            queryClient.setQueryData(dashboardReplaysQueryOptions(5).queryKey, fixture.replays())
        })
        const folders = view.getByRole('list', { name: '최근 수정한 폴더 목록' })
        expect([...folders.querySelectorAll('a')].map((link) => link.textContent)).toEqual([
            'Folder 7',
            'Folder 6',
            'Folder 5',
            'Folder 4',
            'Folder 3',
        ])
        expect(view.getByText('70곡')).toBeTruthy()
        expect(view.getByRole('link', { name: 'Replay chart' }).getAttribute('href')).toBe('/replays/replay-1')
        expect(view.getByRole('link', { name: `제목 미등록 ${SHA.slice(0, 8)}…` }).getAttribute('href')).toBe('/replays/replay-2')
        expect(view.getByText('HARD')).toBeTruthy()
    })

    test('폴더도 리플레이도 없으면 각자 빈 상태를 보인다', () => {
        const view = renderPanel(<MemberFolderReplayPanel />, (queryClient) => {
            queryClient.setQueryData(folderListQueryOptions().queryKey, [])
            queryClient.setQueryData(dashboardReplaysQueryOptions(5).queryKey, { replays: [] })
        })
        expect(view.getByText('폴더가 없습니다.')).toBeTruthy()
        expect(view.getByText('올린 리플레이가 없습니다.')).toBeTruthy()
    })
})

describe('AI 현황 패널', () => {
    const seed = (extra: (queryClient: QueryClient) => void) => (queryClient: QueryClient) => {
        queryClient.setQueryData(aiCredentialListQueryOptions().queryKey, [
            {
                provider: 'anthropic',
                key_hint: 'abcd',
                model: null,
                created_at: fixture.LAST_PLAYED_AT,
                last_validated_at: null,
                last_error_kind: null,
            },
        ])
        queryClient.setQueryData(aiUsageSummaryQueryOptions().queryKey, {
            day: '2026-11-04',
            month: '2026-11',
            today: { calls: 3, input_tokens: 100, output_tokens: 20, cached_input_tokens: 0 },
            this_month: { calls: 9, input_tokens: 1000, output_tokens: 500, cached_input_tokens: 0 },
            limits: { daily_calls: 30, concurrent_runs: 2, monthly_token_limit: null },
        })
        extra(queryClient)
    }

    test('키 등록 여부와 사용량과 판정 습관 한 줄과 게이지 한 줄을 보인다', () => {
        const view = renderPanel(
            <MemberAiStatusPanel />,
            seed((queryClient) => {
                queryClient.setQueryData(aiJudgeHabitQueryOptions().queryKey, judgeHabitView())
                queryClient.setQueryData(aiGaugeAdviceQueryOptions().queryKey, gaugeAdviceView())
            }),
        )
        expect(view.getByText('등록된 키: Anthropic (Claude)')).toBeTruthy()
        expect(view.getByText('3 / 30 회')).toBeTruthy()
        expect(view.getByText('1,500 토큰', { exact: false })).toBeTruthy()
        expect(view.getByText('FAST 55% · 평균 오차 -4.3 ms · 오프셋 제안: 노트가 늦게 보이도록 내리기 (+4.0 ms)')).toBeTruthy()
        expect(view.getByText('실패한 판을 가장 많이 구한 게이지: EASY (4판)')).toBeTruthy()
    })

    test('표본이 모자라면 판정 습관 줄 대신 필요한 데이터를 말한다', () => {
        const view = renderPanel(
            <MemberAiStatusPanel />,
            seed((queryClient) => {
                queryClient.setQueryData(aiJudgeHabitQueryOptions().queryKey, judgeHabitView({ notice: { need: 'recent_plays' } }))
                queryClient.setQueryData(aiGaugeAdviceQueryOptions().queryKey, gaugeAdviceView({ rescues: [] }))
            }),
        )
        expect(view.queryByText('FAST', { exact: false })).toBeNull()
        expect(view.getAllByRole('status')).toHaveLength(1)
        expect(view.getByText('다른 게이지로 구할 수 있었던 판이 없습니다.')).toBeTruthy()
    })

    test('등록된 키가 없으면 등록 링크를 보이고 판정 습관과 게이지는 화면에 들어온 뒤에만 요청한다', () => {
        network.restore()
        network = fixture.stubFetch((url) => (url.endsWith('/ai/credentials') ? fixture.envelope([]) : 'pending'))
        const view = renderPanel(<MemberAiStatusPanel />)
        expect(network.calls.toSorted()).toEqual(['/api/fe/me/ai/credentials', '/api/fe/me/ai/usage'])
        enterView()
        expect(network.calls.toSorted()).toEqual([
            '/api/fe/me/ai/credentials',
            '/api/fe/me/ai/gauge-advice',
            '/api/fe/me/ai/judge-habit',
            '/api/fe/me/ai/usage',
        ])
        expect(view.getByRole('heading', { name: 'AI 현황' })).toBeTruthy()
    })
})

describe('서버 활동 패널', () => {
    test('서버 현황 한 줄과 최근 활동 목록을 보인다', () => {
        const view = renderPanel(<MemberCommunityPanel />, (queryClient) => {
            queryClient.setQueryData(statsSummaryQueryOptions().queryKey, { charts: 5000, players: 12, scores: 99999 })
            queryClient.setQueryData(activityRecentQueryOptions(COMMUNITY_PARAMS).queryKey, [
                fixture.recentRow('Community chart', { player_name: 'Dave', player: { id: 'dave' } }),
            ])
        })
        expect(view.getByText('차트 5,000 · 플레이어 12 · 제출 99,999')).toBeTruthy()
        expect(view.getByRole('link', { name: 'Dave' }).getAttribute('href')).toBe('/players/dave')
        expect(view.getByRole('link', { name: 'Community chart' })).toBeTruthy()
    })

    test('기록이 없으면 안내를 보인다', () => {
        const view = renderPanel(<MemberCommunityPanel />, (queryClient) => {
            queryClient.setQueryData(activityRecentQueryOptions(COMMUNITY_PARAMS).queryKey, [])
        })
        expect(view.getByText('아직 올라온 기록이 없습니다.')).toBeTruthy()
    })
})
