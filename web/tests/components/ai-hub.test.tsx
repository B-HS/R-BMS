import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { click, page, waitFor } = await import('../support/dom-harness')
const { AiReadyGate } = await import('../../src/widgets/ai/ai-ready-gate')
const { AiActiveProviderPanel } = await import('../../src/widgets/ai/ai-active-provider-panel')
const { AiPrincipleNote } = await import('../../src/features/ai/ai-principle-note')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')

type Credential = import('../../src/entities/ai.type').AiCredential

const CREDENTIALS_PATH = '/api/fe/me/ai/credentials'
const SETTINGS_PATH = '/api/fe/me/ai/settings'
const USAGE_PATH = '/api/fe/me/ai/usage'
const NOW = 1_780_000_000_000

const credentialRow = (overrides: Partial<Credential>): Credential => ({
    provider: 'openai',
    key_hint: 'wxyz',
    model: null,
    created_at: NOW,
    last_validated_at: NOW,
    last_error_kind: null,
    ...overrides,
})

const userSetting = (defaultProvider: Credential['provider'] | null = null) => ({
    monthly_token_limit: null,
    default_provider: defaultProvider,
    updated_at: null,
})

const usageSummary = {
    day: '2026-10-19',
    month: '2026-10',
    today: { calls: 12, input_tokens: 0, output_tokens: 0, cached_input_tokens: 0 },
    this_month: { calls: 12, input_tokens: 0, output_tokens: 0, cached_input_tokens: 0 },
    limits: { daily_calls: 30, concurrent_runs: 1, monthly_token_limit: null },
}

let restoreFetch: () => void = () => undefined

const routesFor = (credentials: Credential[], setting = userSetting()) => {
    const installed = installFetchRoutes({
        [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(credentials),
        [`GET ${SETTINGS_PATH}`]: () => jsonResponse(setting),
        [`GET ${USAGE_PATH}`]: () => jsonResponse(usageSummary),
    })
    restoreFetch = installed.restore
    return installed.calls
}

const renderHub = () =>
    renderWithQueryClient(
        <AiReadyGate>
            <AiActiveProviderPanel />
            <p>feature-slot</p>
        </AiReadyGate>,
    )

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

describe('AI 허브 게이트', () => {
    test('키가 없으면 설정의 AI 탭으로 안내하고 기능 자리는 그리지 않는다', async () => {
        routesFor([])
        await renderHub()

        expect(await page.findByText('등록된 AI 키가 없습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: 'AI 키 등록하기' }).getAttribute('href')).toBe('/settings?tab=ai')
        expect(page.queryByText('feature-slot')).toBeNull()
    })

    test('키는 있지만 모델을 고르지 않았으면 모델 선택으로 안내한다', async () => {
        routesFor([credentialRow({ model: null })])
        await renderHub()

        expect(await page.findByText('사용할 모델을 고르지 않았습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: '모델 고르기' }).getAttribute('href')).toBe('/settings?tab=ai')
        expect(page.queryByText('feature-slot')).toBeNull()
    })

    test('서버와 같은 규칙으로 고른 기본 공급자에 모델이 없으면 다른 키에 모델이 있어도 막는다', async () => {
        routesFor(
            [
                credentialRow({ provider: 'anthropic', model: null, created_at: 2 }),
                credentialRow({ provider: 'openai', model: 'gpt-test', created_at: 1 }),
            ],
            userSetting('anthropic'),
        )
        await renderHub()

        expect(await page.findByText('사용할 모델을 고르지 않았습니다')).toBeTruthy()
        expect(page.queryByText('feature-slot')).toBeNull()
    })

    test('쓸 수 있는 키가 있으면 사용 중인 공급자와 모델, 오늘 호출을 보이고 기능 자리를 연다', async () => {
        routesFor([credentialRow({ provider: 'openai', model: 'gpt-test' })])
        await renderHub()

        expect(await page.findByText('feature-slot')).toBeTruthy()
        expect(await page.findByText('gpt-test')).toBeTruthy()
        expect(page.getByText(/OpenAI/)).toBeTruthy()
        expect(await page.findByText('12 / 30 회')).toBeTruthy()
        expect(page.getByRole('link', { name: 'AI 설정' }).getAttribute('href')).toBe('/settings?tab=ai')
    })

    test('화면을 여는 것만으로는 AI 호출이나 쓰기 요청이 나가지 않는다', async () => {
        const calls = routesFor([credentialRow({ provider: 'openai', model: 'gpt-test' })])
        await renderHub()

        await page.findByText('feature-slot')
        await page.findByText('12 / 30 회')
        expect(calls.length).toBeGreaterThan(0)
        expect(calls.every((call) => call.method === 'GET')).toBe(true)
        expect(calls.map((call) => call.path).toSorted()).toEqual([CREDENTIALS_PATH, SETTINGS_PATH, USAGE_PATH].toSorted())
    })

    test('설정을 불러오지 못하면 오류 상태를 보이고 다시 시도하면 이어진다', async () => {
        let attempts = 0
        const installed = installFetchRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => {
                attempts += 1
                return attempts === 1 ? errorResponse(500, 'INTERNAL_ERROR', '서버 오류') : jsonResponse([])
            },
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${USAGE_PATH}`]: () => jsonResponse(usageSummary),
        })
        restoreFetch = installed.restore
        await renderHub()

        expect(await page.findByText('AI 설정을 불러오지 못했습니다')).toBeTruthy()
        await click(page.getByRole('button', { name: '다시 시도' }))
        await waitFor(() => expect(page.getByText('등록된 AI 키가 없습니다')).toBeTruthy())
    })
})

describe('AI 해석의 원칙 안내', () => {
    test('결정론 결과와 AI 해석을 나누고 호출은 직접 요청할 때만이라고 알린다', async () => {
        await renderWithQueryClient(<AiPrincipleNote />)

        expect(page.getByRole('heading', { name: 'AI 해석의 원칙' })).toBeTruthy()
        expect(page.getByText(/숫자, 표, 그래프는 서버가 계산한 결과입니다/)).toBeTruthy()
        expect(page.getByText(/문장 해석만 AI 가 만들며/)).toBeTruthy()
        expect(page.getByText(/AI 는 직접 요청할 때만 호출됩니다/)).toBeTruthy()
    })
})
