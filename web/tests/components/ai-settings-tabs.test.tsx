import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { SearchParamsContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { page } = await import('../support/dom-harness')
const { SettingsTabs } = await import('../../src/widgets/settings/settings-tabs')
const { ProfilePanel } = await import('../../src/widgets/settings/profile-panel')
const { cleanupScreens, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')

type SessionUser = import('../../src/entities/auth.type').SessionUser

const USER: SessionUser = { id: 'user-1', login_id: 'alice', name: 'Alice', email: 'alice@example.test', role: 'user' }
const ROUTER = { push: () => undefined, replace: () => undefined, back: () => undefined, forward: () => undefined, refresh: () => undefined }

let restoreFetch: () => void = () => undefined

const withRouter = (search: string, ui: React.ReactNode) => (
    <AppRouterContext.Provider value={ROUTER as never}>
        <SearchParamsContext.Provider value={new URLSearchParams(search)}>{ui}</SearchParamsContext.Provider>
    </AppRouterContext.Provider>
)

const installAiRoutes = () => {
    const installed = installFetchRoutes({
        'GET /api/fe/me/ai/credentials': () => jsonResponse([]),
        'GET /api/fe/me/ai/settings': () => jsonResponse({ monthly_token_limit: null, default_provider: null, updated_at: null }),
        'GET /api/fe/me/ai/usage': () =>
            jsonResponse({
                day: '2026-10-19',
                month: '2026-10',
                today: { calls: 0, input_tokens: 0, output_tokens: 0, cached_input_tokens: 0 },
                this_month: { calls: 0, input_tokens: 0, output_tokens: 0, cached_input_tokens: 0 },
                limits: { daily_calls: 30, concurrent_runs: 1, monthly_token_limit: null },
            }),
    })
    restoreFetch = installed.restore
    return installed.calls
}

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

describe('설정 탭의 AI 키 탭', () => {
    test('탭 목록 끝에 AI 키 탭이 있다', async () => {
        installAiRoutes()
        await renderWithQueryClient(withRouter('', <SettingsTabs user={USER} />))

        expect(page.getAllByRole('tab').map((tab) => tab.textContent)).toEqual(['프로필', 'API 토큰', '동기화 blob', '라이벌', 'AI 키'])
    })

    test('주소에 tab=ai 가 있으면 AI 키 탭이 열리고 그 탭의 요청만 나간다', async () => {
        const calls = installAiRoutes()
        await renderWithQueryClient(withRouter('tab=ai', <SettingsTabs user={USER} />))

        expect(page.getByRole('tab', { name: 'AI 키' }).getAttribute('aria-selected')).toBe('true')
        expect(await page.findByRole('heading', { name: 'AI 키' })).toBeTruthy()
        expect(await page.findByRole('heading', { name: 'AI 사용량과 설정' })).toBeTruthy()
        expect(page.queryByRole('heading', { name: '프로필' })).toBeNull()
        expect(calls.every((call) => call.path.startsWith('/api/fe/me/ai/'))).toBe(true)
    })

    test('tab 값이 없거나 모르는 값이면 프로필 탭이 열리고 AI 요청은 나가지 않는다', async () => {
        const calls = installAiRoutes()
        await renderWithQueryClient(withRouter('tab=unknown', <SettingsTabs user={USER} />))

        expect(page.getByRole('tab', { name: '프로필' }).getAttribute('aria-selected')).toBe('true')
        expect(await page.findByRole('heading', { name: '프로필' })).toBeTruthy()
        expect(page.queryByRole('heading', { name: 'AI 키' })).toBeNull()
        expect(calls).toHaveLength(0)
    })
})

describe('프로필 패널의 관리자 링크', () => {
    test('관리자에게만 AI 한도 링크를 보인다', async () => {
        installAiRoutes()
        await renderWithQueryClient(withRouter('', <ProfilePanel user={{ ...USER, role: 'admin' }} />))

        expect(page.getByRole('link', { name: 'AI 한도' }).getAttribute('href')).toBe('/admin/ai')
        expect(page.getByRole('link', { name: '빌드 관리' }).getAttribute('href')).toBe('/admin/builds')
    })

    test('일반 사용자에게는 관리자 링크가 없다', async () => {
        installAiRoutes()
        await renderWithQueryClient(withRouter('', <ProfilePanel user={USER} />))

        expect(page.queryByRole('link', { name: 'AI 한도' })).toBeNull()
        expect(page.queryByRole('link', { name: '빌드 관리' })).toBeNull()
    })
})
