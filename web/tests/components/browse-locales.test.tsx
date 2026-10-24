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

const { cleanup, render } = await import('@testing-library/react')
const { HomeIntroPanel } = await import('../../src/widgets/home/home-intro-panel')
const { StatsSummaryPanel } = await import('../../src/widgets/home/stats-summary-panel')
const { PlayerStatsPanel } = await import('../../src/widgets/player/player-stats-panel')
const { PlayerRecentPanel } = await import('../../src/widgets/player/player-recent-panel')
const { LoginForm } = await import('../../src/widgets/auth/login-form')
const { ChartSearchFilter } = await import('../../src/widgets/search/chart-search-filter')
const { CourseCatalogPanel } = await import('../../src/widgets/course/course-catalog-panel')
const { GuideToc } = await import('../../src/widgets/guide/guide-toc')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { withLocale } = await import('../support/browse-intl')

type Locale = import('../../src/shared/i18n/locale').Locale

const LOCALES = ['ko', 'en', 'ja'] as const satisfies readonly Locale[]
const PLAYER_ID = 'alice'
const RECENT_PARAMS = { page: 1, limit: 20 }

const router = { push: () => undefined, replace: () => undefined, back: () => undefined, forward: () => undefined, refresh: () => undefined }

const seededClient = () => new QueryClient({ defaultOptions: { queries: { staleTime: Infinity, retry: false }, mutations: { retry: false } } })

const renderIn = (locale: Locale, ui: React.ReactNode, queryClient = seededClient()) =>
    render(
        <AppRouterContext.Provider value={router as never}>
            <PathnameContext.Provider value='/'>
                <QueryClientProvider client={queryClient}>{withLocale(ui, locale)}</QueryClientProvider>
            </PathnameContext.Provider>
        </AppRouterContext.Provider>,
    )

afterEach(() => cleanup())

describe('홈 위젯의 세 언어 렌더', () => {
    const EXPECTED = {
        ko: { title: 'R-BMS 점수 서버', guide: '연결 방법 보기', chartsLabel: '등록 차트', submissions: '누적 제출' },
        en: { title: 'R-BMS score server', guide: 'How to connect', chartsLabel: 'Registered charts', submissions: 'Total submissions' },
        ja: { title: 'R-BMS スコアサーバー', guide: '接続方法を見る', chartsLabel: '登録譜面', submissions: '累計送信数' },
    } as const

    for (const locale of LOCALES) {
        test(`${locale}: 소개 패널의 제목과 링크`, () => {
            const view = renderIn(locale, <HomeIntroPanel />)
            expect(view.getByText(EXPECTED[locale].title)).toBeTruthy()
            expect(view.getByRole('link', { name: EXPECTED[locale].guide }).getAttribute('href')).toBe('/guide')
            expect(view.getAllByRole('listitem')).toHaveLength(3)
        })

        test(`${locale}: 서버 현황 패널의 라벨과 숫자`, () => {
            const queryClient = seededClient()
            queryClient.setQueryData(QUERY_KEY.STATS.SUMMARY, { charts: 12_345, players: 7, scores: 987_654 })
            const view = renderIn(locale, <StatsSummaryPanel />, queryClient)
            expect(view.getByText(EXPECTED[locale].chartsLabel)).toBeTruthy()
            expect(view.getByText(EXPECTED[locale].submissions)).toBeTruthy()
            expect(view.getByText('12,345')).toBeTruthy()
            expect(view.getByText('987,654')).toBeTruthy()
        })
    }
})

describe('플레이어 위젯의 숫자 문구', () => {
    const seedStats = (queryClient: QueryClient) =>
        queryClient.setQueryData(QUERY_KEY.PLAYER.STATS(PLAYER_ID), {
            total_plays: 2000,
            cleared: 900,
            lamps: [{ clear: 'Hard', assist_grade: 0, count: 1234 }],
            scratch_auto_charts: 5,
        })

    const CLEARED_HINT = {
        ko: '플레이한 차트 1,234개 중 · SCRATCH AUTO 기록 5곡 포함',
        en: 'Out of 1,234 played charts · includes 5 SCRATCH AUTO records',
        ja: 'プレーした譜面1,234個中 · SCRATCH AUTOの記録5曲を含む',
    } as const

    for (const locale of LOCALES) {
        test(`${locale}: 클리어 차트 힌트에 천 단위 구분과 복수형을 쓴다`, () => {
            const queryClient = seededClient()
            seedStats(queryClient)
            const view = renderIn(locale, <PlayerStatsPanel playerId={PLAYER_ID} />, queryClient)
            expect(view.getByText(CLEARED_HINT[locale])).toBeTruthy()
        })
    }

    const recentPage = (total: number) => ({
        data: [],
        pagination: { page: 1, limit: RECENT_PARAMS.limit, total, totalPages: 1 },
    })

    const PLAY_COUNT = {
        ko: { one: '1판', many: '41판' },
        en: { one: '1 play', many: '41 plays' },
        ja: { one: '1プレー', many: '41プレー' },
    } as const

    for (const locale of LOCALES) {
        test(`${locale}: 플레이 횟수 배지는 단수와 복수를 가린다`, () => {
            for (const [total, expected] of [
                [1, PLAY_COUNT[locale].one],
                [41, PLAY_COUNT[locale].many],
            ] as const) {
                const queryClient = seededClient()
                queryClient.setQueryData(QUERY_KEY.PLAYER.RECENT(PLAYER_ID, RECENT_PARAMS), recentPage(total))
                const view = renderIn(locale, <PlayerRecentPanel playerId={PLAYER_ID} params={RECENT_PARAMS} />, queryClient)
                expect(view.getByText(expected)).toBeTruthy()
                cleanup()
            }
        })
    }
})

describe('로그인 폼의 안내 문장', () => {
    test('가이드 안내는 문장 속 링크로 렌더한다', () => {
        for (const [locale, linkName] of [
            ['ko', '가이드'],
            ['en', 'guide'],
            ['ja', 'ガイド'],
        ] as const) {
            const view = renderIn(locale, <LoginForm />)
            expect(view.getByRole('link', { name: linkName }).getAttribute('href')).toBe('/guide')
            cleanup()
        }
    })
})

describe('검색·코스·가이드 위젯의 세 언어 렌더', () => {
    const FILTER = {
        ko: { keyword: '검색어', submit: '검색', mode: '모드', all: '전체' },
        en: { keyword: 'Search keyword', submit: 'Search', mode: 'Mode', all: 'All' },
        ja: { keyword: '検索キーワード', submit: '検索', mode: 'モード', all: 'すべて' },
    } as const

    const COURSE_EMPTY = {
        ko: '등록된 코스가 아직 없습니다',
        en: 'No courses are registered yet',
        ja: 'まだ登録されたコースがありません',
    } as const

    const TOC = {
        ko: { label: '가이드 목차', first: '1. 서버 연결 (Connect)' },
        en: { label: 'Guide table of contents', first: '1. Connect to the server' },
        ja: { label: 'ガイドの目次', first: '1. サーバーに接続 (Connect)' },
    } as const

    for (const locale of LOCALES) {
        test(`${locale}: 차트 검색 필터의 라벨, 버튼, 모드 묶음`, () => {
            const view = renderIn(
                locale,
                <ChartSearchFilter
                    keyword=''
                    mode=''
                    level=''
                    sort='title'
                    sortOptions={[{ value: 'title', label: 'title' }]}
                    columnOptions={[]}
                    activeColumns={[]}
                    summary='0'
                    onChange={() => undefined}
                    onToggleColumn={() => undefined}
                />,
            )
            expect(view.getByLabelText(FILTER[locale].keyword)).toBeTruthy()
            expect(view.getByRole('button', { name: FILTER[locale].submit })).toBeTruthy()
            expect(view.getByLabelText(FILTER[locale].mode).textContent).toContain(FILTER[locale].all)
        })

        test(`${locale}: 코스 목록의 빈 상태`, () => {
            const queryClient = seededClient()
            queryClient.setQueryData(QUERY_KEY.COURSE.CATALOG, { groups: [] })
            const view = renderIn(locale, <CourseCatalogPanel />, queryClient)
            expect(view.getByText(COURSE_EMPTY[locale])).toBeTruthy()
        })

        test(`${locale}: 가이드 목차의 이름과 첫 절 제목`, () => {
            const view = renderIn(locale, <GuideToc />)
            expect(view.getByLabelText(TOC[locale].label)).toBeTruthy()
            expect(view.getByRole('link', { name: TOC[locale].first }).getAttribute('href')).toBe('#connect')
            expect(view.getAllByRole('link')).toHaveLength(9)
        })
    }
})
