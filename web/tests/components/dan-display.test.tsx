import { afterAll, afterEach, describe, expect, mock, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { PathnameContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

mock.module('@shared/lib/auth-client', () => ({
    authClient: { useSession: () => ({ data: null, isPending: false }), $store: { notify: () => undefined } },
}))

const { cleanup, render, within } = await import('@testing-library/react')
const { DanBadgeList } = await import('../../src/features/dan/dan-badge-list')
const { readDisplayedDans } = await import('../../src/entities/dan.type')
const { PlayerLeaderboardPanel } = await import('../../src/widgets/leaderboard/player-leaderboard-panel')
const { PlayerDetailPanel } = await import('../../src/widgets/player/player-detail-panel')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { SidebarProvider } = await import('../../src/shared/ui/sidebar')

type DisplayedDan = import('../../src/entities/dan.type').DisplayedDan
type PlayerLeaderboardRow = import('../../src/entities/stats.type').PlayerLeaderboardRow
type PlayerProfile = import('../../src/entities/player.type').PlayerProfile

const SP_DAN: DisplayedDan = { system: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', label: '発狂五段' }
const STELLA_DAN: DisplayedDan = { system: 'stella', name: 'Stella', label: 'st3' }
const PATHNAME = '/leaderboards'
const LEADERBOARD_PARAMS = { sort: 'cleared', page: 1, limit: 20, include_scratch_auto: false }
const RECENT_PARAMS = { page: 1, limit: 20 }
const PROFILE_ID = 'alice'

const profile = (id: string, extra: PlayerProfile['extra']): PlayerProfile => ({ id, name: id.toUpperCase(), total_plays: 12, rank_points: 0, extra })

const leaderboardRow = (id: string, dans: DisplayedDan[]): PlayerLeaderboardRow => ({
    ...profile(id, { rank: '' }),
    dans,
    cleared_charts: 30,
    played_charts: 40,
})

const router = { push: () => undefined, back: () => undefined, forward: () => undefined, refresh: () => undefined }

const withApp = (queryClient: QueryClient, ui: React.ReactNode) => (
    <AppRouterContext.Provider value={router as never}>
        <PathnameContext.Provider value={PATHNAME}>
            <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
        </PathnameContext.Provider>
    </AppRouterContext.Provider>
)

const seededClient = () => new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } })

afterEach(() => cleanup())

describe('DanBadgeList', () => {
    test('표시할 단위가 없으면 아무것도 그리지 않는다', () => {
        const view = render(<DanBadgeList dans={[]} />)
        expect(view.container.innerHTML).toBe('')
    })

    test('체계마다 한 줄씩 체계 이름과 단위를 함께 보인다', () => {
        const view = render(<DanBadgeList dans={[SP_DAN, STELLA_DAN]} />)
        const list = view.getByLabelText('표시 단위')
        expect(
            within(list)
                .getAllByRole('listitem')
                .map((item) => item.textContent),
        ).toEqual(['GENOSIDE 2018 SP 発狂五段', 'Stella st3'])
    })
})

describe('readDisplayedDans', () => {
    test('프로필 extra 의 dans 를 읽는다', () => {
        expect(readDisplayedDans({ rank: '', dans: [SP_DAN] })).toEqual([SP_DAN])
    })

    test('dans 가 없거나 모양이 다르거나 모르는 체계이면 빈 목록이다', () => {
        for (const extra of [{}, { dans: 'x' }, { dans: [{ system: 'stella' }] }, { dans: [{ system: 'unknown', name: 'n', label: 'l' }] }]) {
            expect(readDisplayedDans(extra)).toEqual([])
        }
    })
})

describe('플레이어 순위 표', () => {
    test('이름 옆에 그 사람이 표시한 단위를 보이고 단위가 없는 사람은 아무것도 붙지 않는다', () => {
        const queryClient = seededClient()
        queryClient.setQueryData(QUERY_KEY.LEADERBOARD.PLAYERS(LEADERBOARD_PARAMS), {
            data: [leaderboardRow('alice', [SP_DAN, STELLA_DAN]), leaderboardRow('bob', [])],
            pagination: { page: 1, limit: 20, total: 2, totalPages: 1 },
        })
        const view = render(withApp(queryClient, <PlayerLeaderboardPanel params={LEADERBOARD_PARAMS} />))

        const [aliceRow, bobRow] = view.getAllByRole('row').slice(1)
        expect(within(aliceRow).getByText('GENOSIDE 2018 SP 発狂五段')).toBeTruthy()
        expect(within(aliceRow).getByText('Stella st3')).toBeTruthy()
        expect(within(bobRow).queryByLabelText('표시 단위')).toBeNull()
        expect(within(aliceRow).getByRole('link', { name: 'ALICE' }).getAttribute('href')).toBe('/players/alice')
    })
})

describe('플레이어 프로필', () => {
    const renderProfile = (extra: PlayerProfile['extra']) => {
        const queryClient = seededClient()
        queryClient.setQueryData(QUERY_KEY.PLAYER.PROFILE(PROFILE_ID), profile(PROFILE_ID, extra))
        queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(PROFILE_ID), { total_plays: 12, cleared: 5, lamps: [], scratch_auto_charts: 0 })
        queryClient.setQueryData(QUERY_KEY.PLAYER.RECENT(PROFILE_ID, RECENT_PARAMS), {
            data: [],
            pagination: { page: 1, limit: 20, total: 0, totalPages: 1 },
        })
        return render(
            withApp(
                queryClient,
                <SidebarProvider>
                    <PlayerDetailPanel playerId={PROFILE_ID} recentParams={RECENT_PARAMS} />
                </SidebarProvider>,
            ),
        )
    }

    test('이름 아래에 extra.dans 의 표시 단위를 보인다', () => {
        const view = renderProfile({ rank: '', dans: [SP_DAN] })
        expect(within(view.getByLabelText('표시 단위')).getByText('GENOSIDE 2018 SP 発狂五段')).toBeTruthy()
    })

    test('옛 서버의 프로필(extra 에 dans 없음)은 단위 줄 없이 그대로 열린다', () => {
        const view = renderProfile({ rank: '' })
        expect(view.getByRole('heading', { name: 'ALICE' })).toBeTruthy()
        expect(view.queryByLabelText('표시 단위')).toBeNull()
    })
})
