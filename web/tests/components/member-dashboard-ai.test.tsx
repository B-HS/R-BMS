import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

mock.module('@shared/lib/auth-client', () => ({
    authClient: {
        useSession: () => ({
            data: { user: { id: 'user-alice', name: 'Alice', email: 'alice@example.test', loginId: 'alice', role: 'user' } },
            isPending: false,
        }),
        $store: { notify: () => undefined },
    },
}))

const { act, cleanup, fireEvent, render } = await import('@testing-library/react')
const fixture = await import('./member-dashboard-fixture')
const trainingFixture = await import('./ai-training-fixture')
const { withLocale } = await import('../support/browse-intl')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { aiProfileQueryOptions, aiTrainingListQueryOptions } = await import('../../src/entities/ai.query')
const { MemberWeaknessPanel } = await import('../../src/widgets/home/member-weakness-panel')
const { MemberTrainingPanel } = await import('../../src/widgets/home/member-training-panel')
const { MemberDashboardGrid } = await import('../../src/widgets/home/member-dashboard-grid')

const { LOGIN_ID, USER_ID } = fixture
const PROFILE_URL = '/api/fe/me/ai/profile'
const TRAINING_URL = '/api/fe/me/ai/training-charts'
const RECENT_PARAMS = { page: 1, limit: 20 }
const MAX_TRAININGS_SHOWN = 5
const SETTLE_ROUNDS = 3

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

const settle = async () => {
    for (let round = 0; round < SETTLE_ROUNDS; round += 1) await act(async () => void (await new Promise<void>((resolve) => setTimeout(resolve, 0))))
}

const renderPanel = (ui: ReactNode, seed: (queryClient: QueryClient) => void = () => undefined) => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false } } })
    seed(queryClient)
    return render(<QueryClientProvider client={queryClient}>{withLocale(ui)}</QueryClientProvider>)
}

const seedProfile = (profile: ReturnType<typeof trainingFixture.trainingProfile>) => (queryClient: QueryClient) => {
    queryClient.setQueryData(aiProfileQueryOptions().queryKey, profile)
}

const seedTrainingList = (list: ReturnType<typeof trainingFixture.trainingList>) => (queryClient: QueryClient) => {
    queryClient.setQueryData(aiTrainingListQueryOptions().queryKey, list)
}

describe('약점 패널', () => {
    test('화면에 들어오기 전에는 프로필을 요청하지 않고 들어오면 한 번 요청한다', () => {
        renderPanel(<MemberWeaknessPanel />)
        expect(network.calls).toEqual([])
        enterView()
        expect(network.calls).toEqual([PROFILE_URL])
    })

    test('가장 센 약점 셋과 훈련곡 링크, 밀도 한계 미니 곡선과 프로필 링크를 보인다', () => {
        const view = renderPanel(<MemberWeaknessPanel />, seedProfile(trainingFixture.trainingProfile()))
        const items = view.getByRole('list', { name: '약점 상위 목록' }).querySelectorAll(':scope > li')
        expect([...items].map((item) => item.querySelector('span')?.textContent)).toEqual(['트릴', '1번 레인', '실패 직전 계단'])
        expect(view.getByRole('link', { name: '트릴 훈련곡 만들기' }).getAttribute('href')).toBe('/ai?tab=training&focus=tag%3Atrill')
        expect(view.getByRole('link', { name: '1번 레인 훈련곡 만들기' }).getAttribute('href')).toBe('/ai?tab=training&focus=lane%3A1')
        expect(view.queryByRole('link', { name: '실패 직전 계단 훈련곡 만들기' })).toBeNull()
        expect(view.queryByText('연타')).toBeNull()
        expect(view.getByRole('link', { name: '약점 프로필 보기' }).getAttribute('href')).toBe('/ai?tab=profile')

        const curve = view.getByRole('img')
        expect(curve.getAttribute('aria-label')).toBe(
            '밀도별 EX 비율 그래프. 편안한 밀도 10 NPS, EX 한계 16 NPS, BP 한계 16 NPS, 친 가장 높은 밀도 16 NPS.',
        )
        expect(curve.querySelectorAll('circle')).toHaveLength(4)
        expect(view.getByText('편안한 밀도 10 NPS · EX 한계 16 NPS')).toBeTruthy()
    })

    test('분석된 판이 없으면 빈 상태만 보인다', () => {
        const profile = trainingFixture.trainingProfile({
            window: { from: 0, to: 1, days: 90, plays: 0, charts: 0, notes: 0, stale: 0, auto_plays: 0, training_plays: 0 },
            weaknesses: [],
            notice: { need: 'traced_plays' },
        })
        const view = renderPanel(<MemberWeaknessPanel />, seedProfile(profile))
        expect(view.getByText('분석된 판이 아직 없습니다. 새 클라이언트로 7키 단일 플레이를 치면 약점이 보입니다.')).toBeTruthy()
        expect(view.queryByRole('img')).toBeNull()
    })

    test('근거 있는 약점이 없고 밀도 곡선도 그릴 수 없으면 각자 이유를 말한다', () => {
        const base = trainingFixture.trainingProfile()
        const profile = trainingFixture.trainingProfile({
            weaknesses: [],
            density: { ...base.density, curve: [], comfort_nps: null, limit_nps_ex: null, limit_nps_bp: null },
            notice: { need: 'traced_plays' },
        })
        const view = renderPanel(<MemberWeaknessPanel />, seedProfile(profile))
        expect(view.getByText('근거가 충분한 약점이 아직 없습니다.')).toBeTruthy()
        expect(view.getByText('밀도 칸마다 노트와 차트가 모자라 밀도 한계를 그릴 수 없습니다.')).toBeTruthy()
        expect(view.getByRole('status').textContent).toContain('판별 데이터')
        expect(view.queryByRole('img')).toBeNull()
    })

    test('프로필을 불러오지 못하면 문구와 다시 시도 단추를 보이고 단추로 다시 요청한다', async () => {
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const view = renderPanel(<MemberWeaknessPanel />)
        enterView()
        await settle()
        expect(view.getByRole('alert').textContent).toContain('약점 프로필을 불러오지 못했습니다.')
        fireEvent.click(view.getByRole('button', { name: '다시 시도' }))
        await settle()
        expect(network.calls.filter((url) => url === PROFILE_URL).length).toBeGreaterThan(1)
    })
})

describe('훈련곡 패널', () => {
    test('화면에 들어오기 전에는 목록을 요청하지 않고 들어오면 한 번 요청한다', () => {
        renderPanel(<MemberTrainingPanel />)
        expect(network.calls).toEqual([])
        enterView()
        expect(network.calls).toEqual([TRAINING_URL])
    })

    test('보관 수와 가장 새 훈련곡 다섯 개를 초점·판 수·최고 EX 와 함께 보이고 상세로 잇는다', () => {
        const charts = Array.from({ length: 7 }, (_, index) =>
            trainingFixture.trainingChart(index + 1, index === 0 ? { plays: 3, best_ex_ratio: 0.877, focus_keys: ['tag:trill', 'lane:1'] } : {}),
        )
        const view = renderPanel(<MemberTrainingPanel />, seedTrainingList(trainingFixture.trainingList({ charts })))
        const items = view.getByRole('list', { name: '최근 훈련곡' }).querySelectorAll(':scope > li')
        expect(items).toHaveLength(MAX_TRAININGS_SHOWN)
        expect(items[0].textContent).toContain('트릴, 1번 레인')
        expect(items[0].textContent).toContain('3판 · 87.7%')
        expect(items[1].textContent).toContain('0판')
        expect(view.getByRole('link', { name: charts[0].title }).getAttribute('href')).toBe(`/ai/training/${charts[0].id}`)
        expect(view.getByText('7/20곡')).toBeTruthy()
        expect(view.getByRole('link', { name: '훈련곡 만들기' }).getAttribute('href')).toBe('/ai?tab=training')
    })

    test('훈련곡이 없으면 안내와 만들기 링크를 보인다', () => {
        const view = renderPanel(<MemberTrainingPanel />, seedTrainingList(trainingFixture.trainingList({ charts: [] })))
        expect(view.getByText('아직 만든 훈련곡이 없습니다.')).toBeTruthy()
        expect(view.getAllByRole('link', { name: '훈련곡 만들기' }).every((link) => link.getAttribute('href') === '/ai?tab=training')).toBe(true)
    })

    test('목록을 불러오지 못하면 문구와 다시 시도 단추를 보인다', async () => {
        network.restore()
        network = fixture.stubFetch(() => fixture.failure())
        const view = renderPanel(<MemberTrainingPanel />)
        enterView()
        await settle()
        expect(view.getByRole('alert').textContent).toContain('훈련곡을 불러오지 못했습니다.')
        expect(view.getByRole('button', { name: '다시 시도' })).toBeTruthy()
    })
})

describe('격자와 최근 플레이의 TRAINING 표지', () => {
    const TRAINED_SHA = trainingFixture.trainingSha256(1)
    const recentRows = [
        fixture.recentRow('Trained play', {
            chart: { md5: 'b'.repeat(32), sha256: TRAINED_SHA, title: 'Trained chart', artist: 'R-BMS', level: 12 },
        }),
        fixture.recentRow('Plain play', {
            chart: { md5: 'c'.repeat(32), sha256: 'f'.repeat(64), title: 'Plain chart', artist: 'someone', level: 12 },
        }),
    ]

    const renderGrid = () => {
        const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false } } })
        queryClient.setQueryData(QUERY_KEY.PLAYER.RECENT(LOGIN_ID, RECENT_PARAMS), {
            data: recentRows,
            pagination: { page: 1, limit: 20, total: recentRows.length, totalPages: 1 },
        })
        return render(
            <QueryClientProvider client={queryClient}>{withLocale(<MemberDashboardGrid userId={USER_ID} loginId={LOGIN_ID} />)}</QueryClientProvider>,
        )
    }

    test('격자가 화면에 들어오기 전에는 훈련곡 목록을 요청하지 않고 들어온 뒤 훈련곡의 판에만 표지를 단다', async () => {
        network.restore()
        network = fixture.stubFetch((url) => (url === TRAINING_URL ? fixture.envelope(trainingFixture.trainingList()) : 'pending'))
        const view = renderGrid()
        expect(network.calls.filter((url) => url === TRAINING_URL)).toEqual([])
        expect(view.queryByText('TRAINING · R-BMS')).toBeNull()

        enterView()
        await settle()
        expect(network.calls.filter((url) => url === TRAINING_URL)).toHaveLength(1)
        expect(view.getByText('TRAINING · R-BMS')).toBeTruthy()
        expect(view.getByText('someone')).toBeTruthy()
        expect(view.queryByText('TRAINING · someone')).toBeNull()
    })

    test('훈련곡 패널과 최근 플레이가 같은 목록 요청을 나눠 쓴다', async () => {
        network.restore()
        network = fixture.stubFetch((url) => (url === TRAINING_URL ? fixture.envelope(trainingFixture.trainingList()) : 'pending'))
        const view = renderGrid()
        enterView()
        await settle()
        expect(network.calls.filter((url) => url === TRAINING_URL)).toHaveLength(1)
        expect(view.getByRole('list', { name: '최근 훈련곡' }).querySelectorAll(':scope > li')).toHaveLength(2)
    })
})
