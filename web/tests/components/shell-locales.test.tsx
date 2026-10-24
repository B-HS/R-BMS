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
const { AppShell } = await import('../../src/widgets/shell/app-shell')
const { ShellPageHeader } = await import('../../src/widgets/shell/shell-page-header')
const { ShellPageLoading } = await import('../../src/widgets/shell/shell-page-loading')
const { SidebarProvider } = await import('../../src/shared/ui/sidebar')
const { TooltipProvider } = await import('../../src/shared/ui/tooltip')
const { withLocale } = await import('../support/browse-intl')

type Locale = import('../../src/shared/i18n/locale').Locale

type ShellLabels = {
    skip: string
    mainNav: string
    home: string
    homeLink: string
    search: string
    guide: string
    theme: string
    language: string
    signIn: string
    openMenu: string
    toggleSidebar: string
    loading: string
}

const LABELS: Record<Locale, ShellLabels> = {
    ko: {
        skip: '본문으로 건너뛰기',
        mainNav: '주 메뉴',
        home: '홈',
        homeLink: 'R-BMS 홈',
        search: '차트 검색',
        guide: '가이드',
        theme: '라이트 · 다크 테마 전환',
        language: '언어',
        signIn: '로그인',
        openMenu: '메뉴 열기',
        toggleSidebar: '사이드바 접기 · 펼치기',
        loading: '순위 불러오는 중',
    },
    en: {
        skip: 'Skip to main content',
        mainNav: 'Main menu',
        home: 'Home',
        homeLink: 'R-BMS home',
        search: 'Chart search',
        guide: 'Guide',
        theme: 'Switch between light and dark theme',
        language: 'Language',
        signIn: 'Sign in',
        openMenu: 'Open menu',
        toggleSidebar: 'Collapse or expand the sidebar',
        loading: 'Loading Rankings',
    },
    ja: {
        skip: '本文へスキップ',
        mainNav: 'メインメニュー',
        home: 'ホーム',
        homeLink: 'R-BMS ホーム',
        search: '譜面検索',
        guide: 'ガイド',
        theme: 'ライト・ダークテーマの切り替え',
        language: '言語',
        signIn: 'ログイン',
        openMenu: 'メニューを開く',
        toggleSidebar: 'サイドバーを折りたたむ・展開する',
        loading: 'ランキングを読み込み中',
    },
}

const PAGE_TITLE: Record<Locale, string> = { ko: '순위', en: 'Rankings', ja: 'ランキング' }

const router = {
    push: () => undefined,
    replace: () => undefined,
    back: () => undefined,
    forward: () => undefined,
    refresh: () => undefined,
    prefetch: () => undefined,
}

const renderShell = (locale: Locale) => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    return render(
        <AppRouterContext.Provider value={router as never}>
            <PathnameContext.Provider value='/search'>
                <QueryClientProvider client={queryClient}>
                    {withLocale(
                        <TooltipProvider>
                            <AppShell>
                                <ShellPageHeader title={PAGE_TITLE[locale]} />
                            </AppShell>
                        </TooltipProvider>,
                        locale,
                    )}
                </QueryClientProvider>
            </PathnameContext.Provider>
        </AppRouterContext.Provider>,
    )
}

afterEach(() => cleanup())

describe('앱 셸의 세 언어 렌더', () => {
    for (const locale of ['ko', 'en', 'ja'] as const) {
        test(`${locale}: 건너뛰기 링크, 주 메뉴, 테마·언어 선택, 로그인 링크를 그 언어로 그린다`, () => {
            const labels = LABELS[locale]
            const view = renderShell(locale)

            expect(view.getByText(labels.skip).getAttribute('href')).toBe('#main-content')
            expect(view.getByLabelText(labels.mainNav).tagName).toBe('NAV')
            expect(view.getByRole('link', { name: labels.home }).getAttribute('href')).toBe('/')
            expect(view.getByRole('link', { name: labels.search }).getAttribute('aria-current')).toBe('page')
            expect(view.getByRole('link', { name: labels.guide }).getAttribute('href')).toBe('/guide')
            expect(view.getByRole('link', { name: labels.homeLink }).getAttribute('href')).toBe('/')
            expect(view.getByRole('button', { name: labels.theme })).toBeTruthy()
            expect(view.getByRole('button', { name: labels.language })).toBeTruthy()
            expect(view.getByRole('link', { name: labels.signIn }).getAttribute('href')).toBe('/login?next=%2Fsearch')
            expect(view.getByRole('button', { name: labels.openMenu })).toBeTruthy()
            expect(view.getByRole('button', { name: labels.toggleSidebar })).toBeTruthy()
        })
    }

    test('언어 선택은 테마 버튼 바로 옆, 계정 칸 앞에 놓이고 바닥 격자는 세 칸이다', () => {
        const view = renderShell('ko')
        const theme = view.getByRole('button', { name: LABELS.ko.theme })
        const language = view.getByRole('button', { name: LABELS.ko.language })
        const account = view.getByRole('link', { name: LABELS.ko.signIn })

        expect(theme.compareDocumentPosition(language) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
        expect(language.compareDocumentPosition(account) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
        expect(language.closest('ul')?.className).toContain('grid-cols-[auto_auto_minmax(0,1fr)]')
        expect(theme.closest('ul')).toBe(language.closest('ul'))
        expect(account.closest('ul')).toBe(language.closest('ul'))
    })

    for (const locale of ['ko', 'en', 'ja'] as const) {
        test(`${locale}: 로딩 쪽 제목 줄은 제목 자리표를 넣은 문구를 읽어 준다`, () => {
            const view = render(
                withLocale(
                    <SidebarProvider>
                        <ShellPageLoading title={PAGE_TITLE[locale]} />
                    </SidebarProvider>,
                    locale,
                ),
            )

            expect(view.getByRole('status').getAttribute('aria-label')).toBe(LABELS[locale].loading)
        })
    }
})
