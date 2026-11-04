import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const session: { user: { id: string; name: string; email: string; loginId: string; role: string } | null; isPending: boolean } = {
    user: null,
    isPending: false,
}

mock.module('@shared/lib/auth-client', () => ({
    authClient: {
        useSession: () => ({ data: session.user ? { user: session.user } : null, isPending: session.isPending }),
        $store: { notify: () => undefined },
    },
}))

const { cleanup, render } = await import('@testing-library/react')
const fixture = await import('./member-dashboard-fixture')
const { withLocale } = await import('../support/browse-intl')
const { MemberDashboardGrid } = await import('../../src/widgets/home/member-dashboard-grid')
const { removeUserScopedQueries } = await import('../../src/entities/auth.query')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { DASHBOARD_FILL_PANEL_CLASS_NAME, DASHBOARD_FULL_PANEL_CLASS_NAME, DASHBOARD_WIDE_PANEL_CLASS_NAME } =
    await import('../../src/shared/constants/dashboard')

const { LOGIN_ID, USER_ID } = fixture
const PANEL_COUNT = 13
const SIGNED_IN_USER = { id: USER_ID, name: 'Alice', email: 'alice@example.test', loginId: LOGIN_ID, role: 'user' }

let network: ReturnType<typeof fixture.stubFetch>
let observer: ReturnType<typeof fixture.installIntersectionObserver>

beforeEach(() => {
    session.user = SIGNED_IN_USER
    session.isPending = false
    network = fixture.stubFetch(() => 'pending')
    observer = fixture.installIntersectionObserver()
})

afterEach(() => {
    cleanup()
    network.restore()
    observer.restore()
})

const renderGrid = () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false } } })
    return render(
        <QueryClientProvider client={queryClient}>{withLocale(<MemberDashboardGrid userId={USER_ID} loginId={LOGIN_ID} />)}</QueryClientProvider>,
    )
}

const spanOf = (panel: Element, columns: number) => {
    if (panel.classList.contains(DASHBOARD_FULL_PANEL_CLASS_NAME)) return columns
    const isWide = panel.classList.contains(DASHBOARD_WIDE_PANEL_CLASS_NAME)
    const isFill = panel.classList.contains(DASHBOARD_FILL_PANEL_CLASS_NAME)
    if (isWide && columns >= 3) return 2
    if (isFill && columns >= 4) return 2
    return 1
}

/** Places the spans the way a dense grid does (each one in the first free run of cells from the top left) and counts the cells left empty above the last used cell. */
const emptyCellsOf = (spans: number[], columns: number) => {
    const rows: boolean[][] = []
    for (const span of spans) {
        for (let row = 0; ; row += 1) {
            rows[row] ??= Array.from({ length: columns }, () => false)
            const start = Array.from({ length: columns - span + 1 }, (_, index) => index).find((index) =>
                rows[row].slice(index, index + span).every((taken) => !taken),
            )
            if (start === undefined) continue
            for (let cell = start; cell < start + span; cell += 1) rows[row][cell] = true
            break
        }
    }
    return rows.flat().filter((taken) => !taken).length
}

describe('회원 대시보드 격자', () => {
    test('서버가 본 사용자와 세션이 같으면 패널 열세 개를 읽는 순서대로 그린다', () => {
        const view = renderGrid()
        expect([...view.container.querySelectorAll('h2')].map((heading) => heading.textContent)).toEqual([
            '내 정보',
            '플레이 요약',
            '램프 현황',
            '표별 진행',
            '활동',
            '최근 플레이',
            '단위·코스',
            '이번 주',
            '목표',
            '라이벌',
            '폴더·리플레이',
            'AI 현황',
            '서버 활동',
        ])
    })

    test('세션이 아직 확인 중이면 서버가 그린 대로 패널을 그린다', () => {
        session.user = null
        session.isPending = true
        const view = renderGrid()
        expect(view.container.querySelectorAll('h2')).toHaveLength(PANEL_COUNT)
    })

    test('다른 탭에서 로그아웃하거나 다른 사람으로 바뀌면 패널 대신 스켈레톤을 두고 요청하지 않는다', () => {
        session.user = null
        const signedOut = renderGrid()
        expect(signedOut.container.querySelectorAll('h2')).toHaveLength(0)
        expect(signedOut.getAllByRole('status')).toHaveLength(4)
        expect(network.calls).toEqual([])
        cleanup()
        session.user = { ...SIGNED_IN_USER, id: 'someone-else' }
        const switched = renderGrid()
        expect(switched.container.querySelectorAll('h2')).toHaveLength(0)
        expect(network.calls).toEqual([])
    })

    test('패널이 청하는 폭은 1열에서 4열까지 어느 열 수에서도 빈 칸을 남기지 않는다', () => {
        const view = renderGrid()
        const panels = [...(view.container.firstElementChild?.children ?? [])]
        expect(panels).toHaveLength(PANEL_COUNT)
        const fullPanel = panels.at(-1)
        expect(fullPanel?.classList.contains(DASHBOARD_FULL_PANEL_CLASS_NAME)).toBe(true)
        const gridded = panels.slice(0, -1)
        for (const columns of [1, 2, 3, 4]) {
            expect({
                columns,
                emptyCells: emptyCellsOf(
                    gridded.map((panel) => spanOf(panel, columns)),
                    columns,
                ),
            }).toEqual({ columns, emptyCells: 0 })
        }
    })

    test('빈 칸 계산은 조밀 배치로 뒤의 작은 패널이 구멍을 메우는 것까지 따진다', () => {
        expect(emptyCellsOf([2, 2], 3)).toBe(2)
        expect(emptyCellsOf([2, 2, 1, 1], 3)).toBe(0)
        expect(emptyCellsOf([1, 2, 1], 4)).toBe(0)
        expect(emptyCellsOf([1, 2, 2], 4)).toBe(3)
    })

    test('패널을 뷰포트 밖에 둔 채로는 화면에 들어와야 하는 AI 요청이 하나도 나가지 않는다', () => {
        renderGrid()
        expect(network.calls.filter((url) => url.includes('/ai/weekly') || url.includes('/ai/recommend') || url.includes('/ai/goals'))).toEqual([])
        expect(
            network.calls.filter((url) => url.includes('/dashboard/rivals') || url.includes('/ai/judge-habit') || url.includes('/ai/gauge-advice')),
        ).toEqual([])
    })

    test('로그인·로그아웃 때 비우는 사용자 범위 캐시에 대시보드와 내 단위가 들어 있고 공개 플레이어 캐시는 남는다', () => {
        const queryClient = new QueryClient()
        queryClient.setQueryData(QUERY_KEY.DASHBOARD.OVERVIEW, { owner: 'alice' })
        queryClient.setQueryData(QUERY_KEY.DASHBOARD.CALENDAR(182), { owner: 'alice' })
        queryClient.setQueryData(QUERY_KEY.DAN.MINE, { owner: 'alice' })
        queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(LOGIN_ID), { owner: 'public' })
        removeUserScopedQueries(queryClient)
        expect(queryClient.getQueryData(QUERY_KEY.DASHBOARD.OVERVIEW)).toBeUndefined()
        expect(queryClient.getQueryData(QUERY_KEY.DASHBOARD.CALENDAR(182))).toBeUndefined()
        expect(queryClient.getQueryData(QUERY_KEY.DAN.MINE)).toBeUndefined()
        expect(queryClient.getQueryData<{ owner: string }>(QUERY_KEY.PLAYER.STATS(LOGIN_ID))).toEqual({ owner: 'public' })
    })
})
