import { afterAll, afterEach, describe, expect, mock, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { PathnameContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const session: { user: { id: string; name: string; email: string; loginId: string; role: string } | null } = { user: null }

mock.module('@shared/lib/auth-client', () => ({
    authClient: { useSession: () => ({ data: session.user ? { user: session.user } : null, isPending: false }), $store: { notify: () => undefined } },
}))

const { cleanup, fireEvent, render } = await import('@testing-library/react')
const { PlayerRecentPanel } = await import('../../src/widgets/player/player-recent-panel')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')

const PLAYER_ID = 'alice'
const PATHNAME = '/players/alice'
const LIMIT = 20
const SHA = 'a'.repeat(64)

type RecentRow = import('../../src/entities/player.type').PlayerRecentRow

const recentRow = (scoreId: string): RecentRow => ({
    score_id: scoreId,
    player: { id: PLAYER_ID },
    player_name: 'Alice',
    clear: 'LightAssistEasy',
    ex_score: 1200,
    minbp: 4,
    max_combo: 300,
    played_at: 1_780_000_000_000,
    scratch_auto: true,
    chart: { md5: 'b'.repeat(32), sha256: SHA, title: scoreId, artist: 'artist', level: 12 },
})

const renderPanel = (params: { page: number; limit: number }, page: { data: RecentRow[]; total: number }) => {
    const pushed: string[] = []
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } })
    queryClient.setQueryData(QUERY_KEY.PLAYER.RECENT(PLAYER_ID, params), {
        data: page.data,
        pagination: { page: params.page, limit: params.limit, total: page.total, totalPages: Math.max(1, Math.ceil(page.total / params.limit)) },
    })
    const router = { push: (href: string) => void pushed.push(href), back: () => undefined, forward: () => undefined, refresh: () => undefined }
    const view = render(
        <AppRouterContext.Provider value={router as never}>
            <PathnameContext.Provider value={PATHNAME}>
                <QueryClientProvider client={queryClient}>
                    <PlayerRecentPanel playerId={PLAYER_ID} params={params} />
                </QueryClientProvider>
            </PathnameContext.Provider>
        </AppRouterContext.Provider>,
    )
    return { pushed, view }
}

afterEach(() => cleanup())

describe('플레이어 플레이 기록 패널', () => {
    test('제목은 플레이 기록이고 전체 판 수와 페이지를 보여 준다', () => {
        const { view } = renderPanel({ page: 2, limit: LIMIT }, { data: [recentRow('Second page chart')], total: 41 })
        expect(view.getByRole('heading', { name: '플레이 기록' })).toBeTruthy()
        expect(view.getByText('41판')).toBeTruthy()
        expect(view.getByText('2 / 3')).toBeTruthy()
        expect(view.getByText('Second page chart')).toBeTruthy()
        expect(view.getByLabelText('플레이어 플레이 기록')).toBeTruthy()
        expect(view.queryByText('최근 기록')).toBeNull()
    })

    test('다음 페이지는 page 를 주소에 싣고 첫 페이지로 가면 page 를 지운다', () => {
        const { pushed, view } = renderPanel({ page: 2, limit: LIMIT }, { data: [recentRow('Second page chart')], total: 41 })
        fireEvent.click(view.getByLabelText('다음 페이지'))
        fireEvent.click(view.getByLabelText('이전 페이지'))
        expect(pushed).toEqual([`${PATHNAME}?page=3`, PATHNAME])
    })

    test('기록이 하나도 없으면 빈 상태이고 페이지 이동은 없다', () => {
        const { view } = renderPanel({ page: 1, limit: LIMIT }, { data: [], total: 0 })
        expect(view.getByText('아직 기록이 없습니다.')).toBeTruthy()
        expect(view.queryByLabelText('페이지 이동')).toBeNull()
    })

    test('범위를 벗어난 페이지는 마지막 페이지로 돌아가는 길을 준다', () => {
        const { pushed, view } = renderPanel({ page: 9, limit: LIMIT }, { data: [], total: 41 })
        expect(view.getByText('이 페이지에는 기록이 없습니다.')).toBeTruthy()
        fireEvent.click(view.getByText('마지막 페이지로'))
        expect(pushed).toEqual([`${PATHNAME}?page=3`])
    })

    test('로그인한 본인의 기록에만 그 판의 코치 페이지 링크를 단다', async () => {
        session.user = { id: 'user-bob', name: 'Bob', email: 'bob@example.test', loginId: 'bob', role: 'user' }
        const other = renderPanel({ page: 1, limit: LIMIT }, { data: [recentRow('Other chart')], total: 1 })
        expect(other.view.queryByRole('link', { name: '판 코치' })).toBeNull()
        cleanup()
        session.user = { id: 'user-alice', name: 'Alice', email: 'alice@example.test', loginId: PLAYER_ID, role: 'user' }
        const own = renderPanel({ page: 1, limit: LIMIT }, { data: [recentRow('sc_own')], total: 1 })
        const link = await own.view.findByRole('link', { name: '판 코치' })
        expect(link.getAttribute('href')).toBe('/ai/plays/sc_own')
        session.user = null
    })
})
