import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { changeValue, click, keyDown, page, submit, waitFor, within } = await import('../support/dom-harness')
const { toast } = await import('sonner')
const { AiCredentialPanel } = await import('../../src/widgets/settings/ai-credential-panel')
const { AiUsagePanel } = await import('../../src/widgets/settings/ai-usage-panel')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, noContentResponse, renderWithQueryClient, toastTexts } =
    await import('../support/ai-screen')

type Credential = import('../../src/entities/ai.type').AiCredential
type FetchRoutes = Parameters<typeof installFetchRoutes>[0]

const CREDENTIALS_PATH = '/api/fe/me/ai/credentials'
const SETTINGS_PATH = '/api/fe/me/ai/settings'
const USAGE_PATH = '/api/fe/me/ai/usage'
const FAKE_KEY = 'sk-test-0123456789-ABCDwxyz'
const NOW = 1_780_000_000_000

const credentialRow = (overrides: Partial<Credential> = {}): Credential => ({
    provider: 'openai',
    key_hint: 'wxyz',
    model: null,
    created_at: NOW - 86_400_000,
    last_validated_at: NOW,
    last_error_kind: null,
    ...overrides,
})

type UserSettingRow = { monthly_token_limit: number | null; default_provider: Credential['provider'] | null; updated_at: number | null }

const userSetting = (overrides: Partial<UserSettingRow> = {}): UserSettingRow => ({
    monthly_token_limit: null,
    default_provider: null,
    updated_at: null,
    ...overrides,
})

const totals = (calls: number, input: number, output: number, cached: number) => ({
    calls,
    input_tokens: input,
    output_tokens: output,
    cached_input_tokens: cached,
})

const usageSummary = (overrides: Partial<{ monthly_token_limit: number | null }> = {}) => ({
    day: '2026-10-19',
    month: '2026-10',
    today: totals(12, 3000, 800, 500),
    this_month: totals(40, 12_000, 3_500, 1_000),
    limits: { daily_calls: 30, concurrent_runs: 1, monthly_token_limit: null, ...overrides },
})

const openSelect = (trigger: HTMLElement) => keyDown(trigger, 'ArrowDown')

let restoreFetch: () => void = () => undefined

const useRoutes = (routes: FetchRoutes) => {
    const installed = installFetchRoutes(routes)
    restoreFetch = installed.restore
    return installed.calls
}

const card = (name: string) => page.getByRole('region', { name })

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
    toast.dismiss()
})

describe('설정 AI 키 패널 - 목록', () => {
    test('키가 없으면 세 공급자 카드가 모두 미등록이고 비밀번호 입력란을 보여 준다', async () => {
        useRoutes({ [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([]), [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()) })
        await renderWithQueryClient(<AiCredentialPanel />)

        for (const name of ['Anthropic (Claude)', 'OpenAI', 'Ollama Cloud']) {
            const providerCard = await page.findByRole('region', { name })
            expect(within(providerCard).getByText('미등록')).toBeTruthy()
        }
        const input = page.getByLabelText('OpenAI API 키') as HTMLInputElement
        expect([input.type, input.autocomplete]).toEqual(['password', 'off'])
        expect(page.getByText(/키는 이 서버에 암호화해 보관하고 AI 호출에만 씁니다/)).toBeTruthy()
        expect(page.getByText(/지출 한도를 걸어 두기를 권합니다/)).toBeTruthy()
    })

    test('등록된 키는 끝 4자만 보이고 모델을 고르지 않았다고 알린다', async () => {
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([credentialRow()]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        const openai = await page.findByRole('region', { name: 'OpenAI' })
        expect(within(openai).getByText('••••wxyz')).toBeTruthy()
        expect(within(openai).getByText('등록됨')).toBeTruthy()
        expect(within(openai).getByText('사용 중')).toBeTruthy()
        expect(within(openai).getByText('선택 안 됨')).toBeTruthy()
        expect(within(openai).getByText('모델을 고르지 않으면 이 키로 AI 를 호출할 수 없습니다.')).toBeTruthy()
        expect(within(openai).queryByLabelText('OpenAI API 키')).toBeNull()
        expect(within(card('Anthropic (Claude)')).queryByText('사용 중')).toBeNull()
    })

    test('기본 공급자가 정해져 있으면 그 카드에 사용 중이 붙는다', async () => {
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () =>
                jsonResponse([
                    credentialRow({ provider: 'openai', created_at: 1 }),
                    credentialRow({ provider: 'anthropic', key_hint: 'abcd', created_at: 2 }),
                ]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting({ default_provider: 'anthropic' })),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await page.findByText('••••abcd')
        await waitFor(() => expect(within(card('Anthropic (Claude)')).getByText('사용 중')).toBeTruthy())
        expect(within(card('OpenAI')).queryByText('사용 중')).toBeNull()
    })

    test('마지막 검증이 실패했으면 실패 종류를 배지로 보여 준다', async () => {
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([credentialRow({ last_validated_at: null, last_error_kind: 'invalid_key' })]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        expect(await within(await page.findByRole('region', { name: 'OpenAI' })).findByText('키가 거부됨')).toBeTruthy()
    })

    test('목록 요청이 실패하면 오류 상태와 다시 시도 버튼을 보여 준다', async () => {
        let attempts = 0
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => {
                attempts += 1
                return attempts === 1 ? errorResponse(503, 'SERVICE_NOT_CONFIGURED', '서비스가 준비되지 않았습니다.') : jsonResponse([])
            },
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        expect(await page.findByText('AI 키 목록을 불러오지 못했습니다')).toBeTruthy()
        await click(page.getByRole('button', { name: '다시 시도' }))
        expect(await page.findByRole('region', { name: 'OpenAI' })).toBeTruthy()
    })
})

describe('설정 AI 키 패널 - 키 등록', () => {
    const registerRoutes = (stored: Credential[], onPut?: () => Response) =>
        ({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(stored),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`PUT ${CREDENTIALS_PATH}/openai`]: () => {
                if (onPut) return onPut()
                stored.push(credentialRow())
                return jsonResponse(credentialRow())
            },
        }) satisfies FetchRoutes

    test('제출하면 PUT 으로 키를 보내고 입력란을 바로 비운 뒤 목록을 다시 받는다', async () => {
        const calls = useRoutes(registerRoutes([]))
        const { queryClient } = await renderWithQueryClient(<AiCredentialPanel />)

        const input = (await page.findByLabelText('OpenAI API 키')) as HTMLInputElement
        input.value = `  ${FAKE_KEY}  `
        await submit(input.closest('form') as HTMLFormElement)

        expect(input.value).toBe('')
        await waitFor(() => expect(within(card('OpenAI')).getByText('••••wxyz')).toBeTruthy())
        const put = calls.find((call) => call.method === 'PUT')
        expect([put?.path, put?.body]).toEqual([`${CREDENTIALS_PATH}/openai`, { api_key: FAKE_KEY }])
        expect(calls.filter((call) => call.method === 'GET' && call.path === CREDENTIALS_PATH).length).toBeGreaterThanOrEqual(2)
        expect(toastTexts()).toContain('OpenAI 키를 저장했습니다. 모델을 고르면 AI 기능에 쓸 수 있습니다.')
        expect(document.body.innerHTML).not.toContain(FAKE_KEY)
        expect(
            JSON.stringify(
                queryClient
                    .getQueryCache()
                    .getAll()
                    .map((query) => query.state.data),
            ),
        ).not.toContain(FAKE_KEY)
    })

    test('끝난 등록 요청은 키가 든 변수와 함께 뮤테이션 캐시에서 사라진다', async () => {
        useRoutes(registerRoutes([]))
        const { queryClient } = await renderWithQueryClient(<AiCredentialPanel />)

        const input = (await page.findByLabelText('OpenAI API 키')) as HTMLInputElement
        input.value = FAKE_KEY
        await submit(input.closest('form') as HTMLFormElement)

        await waitFor(() => expect(within(card('OpenAI')).getByText('••••wxyz')).toBeTruthy())
        await waitFor(() => expect(queryClient.getMutationCache().getAll()).toHaveLength(0))
        expect(JSON.stringify(queryClient.getMutationCache().getAll())).not.toContain(FAKE_KEY)
    })

    test('서버가 키를 거절하면 서버 메시지를 토스트로 알리고 키는 어디에도 남지 않는다', async () => {
        const message = 'AI 키가 맞지 않거나 취소되었습니다. 설정에서 다시 등록해 주세요.'
        useRoutes(registerRoutes([], () => errorResponse(400, 'AI_CREDENTIAL_INVALID', message)))
        const { queryClient } = await renderWithQueryClient(<AiCredentialPanel />)

        const input = (await page.findByLabelText('OpenAI API 키')) as HTMLInputElement
        input.value = FAKE_KEY
        await submit(input.closest('form') as HTMLFormElement)

        await waitFor(() => expect(toastTexts()).toContain(message))
        await waitFor(() => expect(queryClient.getMutationCache().getAll()).toHaveLength(0))
        expect(input.value).toBe('')
        expect(within(card('OpenAI')).getByText('미등록')).toBeTruthy()
        expect(toastTexts().join('\n')).not.toContain(FAKE_KEY)
        expect(document.body.innerHTML).not.toContain(FAKE_KEY)
    })

    test('너무 짧은 키는 요청을 보내지 않는다', async () => {
        const calls = useRoutes(registerRoutes([]))
        await renderWithQueryClient(<AiCredentialPanel />)

        const input = (await page.findByLabelText('OpenAI API 키')) as HTMLInputElement
        input.value = 'short'
        await submit(input.closest('form') as HTMLFormElement)

        expect(calls.some((call) => call.method === 'PUT')).toBe(false)
        expect(input.minLength).toBe(8)
        expect(input.required).toBe(true)
    })

    test('키 교체는 새 입력란을 열고 취소하면 닫는다', async () => {
        useRoutes(registerRoutes([credentialRow()]))
        await renderWithQueryClient(<AiCredentialPanel />)

        const openai = await page.findByRole('region', { name: 'OpenAI' })
        await click(await within(openai).findByRole('button', { name: 'OpenAI 키 교체' }))
        expect(within(openai).getByLabelText('새 OpenAI API 키')).toBeTruthy()
        expect(within(openai).queryByRole('button', { name: 'OpenAI 키 다시 검증' })).toBeNull()
        await click(within(openai).getByRole('button', { name: '취소' }))
        expect(within(openai).queryByLabelText('새 OpenAI API 키')).toBeNull()
    })
})

describe('설정 AI 키 패널 - 검증과 삭제', () => {
    test('다시 검증은 POST validate 를 보내고 성공을 알린다', async () => {
        const calls = useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([credentialRow()]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`POST ${CREDENTIALS_PATH}/openai/validate`]: () => jsonResponse(credentialRow()),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 키 다시 검증' }))

        await waitFor(() => expect(toastTexts()).toContain('OpenAI 키가 유효합니다.'))
        expect(calls.some((call) => call.method === 'POST' && call.path === `${CREDENTIALS_PATH}/openai/validate`)).toBe(true)
    })

    test('검증이 실패해도 서버가 기록한 실패 종류를 다시 받아 보여 준다', async () => {
        let isRejected = false
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () =>
                jsonResponse([credentialRow(isRejected ? { last_validated_at: null, last_error_kind: 'invalid_key' } : {})]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`POST ${CREDENTIALS_PATH}/openai/validate`]: () => {
                isRejected = true
                return errorResponse(400, 'AI_CREDENTIAL_INVALID', 'AI 키가 맞지 않거나 취소되었습니다. 설정에서 다시 등록해 주세요.')
            },
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 키 다시 검증' }))

        expect(await within(card('OpenAI')).findByText('키가 거부됨')).toBeTruthy()
        expect(toastTexts()).toContain('AI 키가 맞지 않거나 취소되었습니다. 설정에서 다시 등록해 주세요.')
    })

    test('삭제는 확인을 거친 뒤에만 DELETE 를 보낸다', async () => {
        const stored = [credentialRow({ model: 'gpt-test' })]
        const calls = useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(stored),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`DELETE ${CREDENTIALS_PATH}/openai`]: () => {
                stored.length = 0
                return noContentResponse()
            },
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 키 삭제' }))
        const dialog = await page.findByRole('alertdialog')
        expect(within(dialog).getByText('OpenAI 키를 삭제할까요?')).toBeTruthy()
        expect(calls.some((call) => call.method === 'DELETE')).toBe(false)

        await click(within(dialog).getByRole('button', { name: '삭제' }))

        await waitFor(() => expect(within(card('OpenAI')).getByText('미등록')).toBeTruthy())
        expect(calls.filter((call) => call.method === 'DELETE')).toHaveLength(1)
        expect(toastTexts()).toContain('OpenAI 키를 삭제했습니다.')
    })
})

describe('설정 AI 키 패널 - 모델 선택', () => {
    const MODELS_PATH = `${CREDENTIALS_PATH}/openai/models`
    const models = [
        { id: 'gpt-alpha', label: 'gpt-alpha', structured: null },
        { id: 'gpt-beta', label: 'gpt-beta', structured: null },
    ]

    test('목록을 열기 전에는 공급자에게 모델 목록을 묻지 않는다', async () => {
        const calls = useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([credentialRow()]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${MODELS_PATH}`]: () => jsonResponse(models),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await page.findByText('••••wxyz')
        expect(calls.some((call) => call.path.endsWith('/models'))).toBe(false)
    })

    test('목록에서 모델을 고르면 PATCH 로 저장하고 닫는다', async () => {
        const stored = [credentialRow()]
        const calls = useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(stored),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${MODELS_PATH}`]: () => jsonResponse(models),
            [`PATCH ${CREDENTIALS_PATH}/openai`]: () => {
                stored[0] = credentialRow({ model: 'gpt-beta' })
                return jsonResponse(stored[0])
            },
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 모델 선택' }))
        await openSelect(await within(card('OpenAI')).findByRole('combobox', { name: '모델' }))
        await click(await page.findByRole('option', { name: /gpt-beta/ }))

        await waitFor(() => expect(within(card('OpenAI')).getByText('gpt-beta')).toBeTruthy())
        const patch = calls.find((call) => call.method === 'PATCH')
        expect([patch?.path, patch?.body]).toEqual([`${CREDENTIALS_PATH}/openai`, { model: 'gpt-beta' }])
        expect(toastTexts()).toContain('OpenAI 모델을 gpt-beta 로 바꿨습니다.')
        expect(within(card('OpenAI')).queryByRole('combobox', { name: '모델' })).toBeNull()
    })

    test('구조화 출력 지원을 확인하지 못한 모델은 그렇게 표시한다', async () => {
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([credentialRow()]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${MODELS_PATH}`]: () => jsonResponse(models),
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 모델 선택' }))
        await openSelect(await within(card('OpenAI')).findByRole('combobox', { name: '모델' }))

        expect(await page.findByRole('option', { name: 'gpt-alpha · 형식 지원 확인 안 됨' })).toBeTruthy()
    })

    test('모델 목록 조회가 실패하면 서버 메시지와 다시 시도를 보여 주고 다시 요청한다', async () => {
        let attempts = 0
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([credentialRow()]),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${MODELS_PATH}`]: () => {
                attempts += 1
                return attempts === 1
                    ? errorResponse(502, 'AI_PROVIDER_UNAVAILABLE', 'AI 공급자에 연결하지 못했습니다. 잠시 뒤 다시 시도해 주세요.')
                    : jsonResponse(models)
            },
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 모델 선택' }))

        expect(await page.findByText('모델 목록을 불러오지 못했습니다')).toBeTruthy()
        expect(page.getByText('AI 공급자에 연결하지 못했습니다. 잠시 뒤 다시 시도해 주세요.')).toBeTruthy()
        await click(page.getByRole('button', { name: '다시 시도' }))
        expect(await within(card('OpenAI')).findByRole('combobox', { name: '모델' })).toBeTruthy()
        expect(attempts).toBe(2)
    })
})

describe('설정 AI 키 패널 - 403 으로 막힌 키', () => {
    const MODELS_PATH = `${CREDENTIALS_PATH}/openai/models`
    const FORBIDDEN_MESSAGE = 'AI 공급자가 이 키의 요청을 막았습니다. 키 권한 또는 모델 접근을 확인하세요.'

    test('등록은 되지만 경고 토스트와 카드 배지로 키 권한 확인을 알린다', async () => {
        const stored: Credential[] = []
        useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(stored),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`PUT ${CREDENTIALS_PATH}/openai`]: () => {
                stored.push(credentialRow({ last_error_kind: 'forbidden' }))
                return jsonResponse(stored[0])
            },
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        const input = (await page.findByLabelText('OpenAI API 키')) as HTMLInputElement
        input.value = FAKE_KEY
        await submit(input.closest('form') as HTMLFormElement)

        await waitFor(() => expect(toastTexts().some((text) => text.includes('키 권한 또는 모델 접근을 확인하세요'))).toBe(true))
        expect(await within(card('OpenAI')).findByText('키 권한 또는 모델 접근을 확인하세요')).toBeTruthy()
    })

    test('모델 목록을 받지 못하면 모델 id 를 직접 적어 저장할 수 있다', async () => {
        const stored = [credentialRow()]
        const calls = useRoutes({
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(stored),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${MODELS_PATH}`]: () => errorResponse(403, 'AI_PROVIDER_FORBIDDEN', FORBIDDEN_MESSAGE, { reason: 'forbidden' }),
            [`PATCH ${CREDENTIALS_PATH}/openai`]: () => {
                stored[0] = credentialRow({ model: 'gpt-typed-1' })
                return jsonResponse(stored[0])
            },
        })
        await renderWithQueryClient(<AiCredentialPanel />)

        await click(await page.findByRole('button', { name: 'OpenAI 모델 선택' }))
        expect(await page.findByText(FORBIDDEN_MESSAGE)).toBeTruthy()
        const input = (await page.findByLabelText('OpenAI 모델 id 직접 입력')) as HTMLInputElement
        await changeValue(input, '  gpt-typed-1 ')
        await submit(input.closest('form') as HTMLFormElement)

        await waitFor(() => expect(within(card('OpenAI')).getByText('gpt-typed-1')).toBeTruthy())
        const patch = calls.find((call) => call.method === 'PATCH')
        expect([patch?.path, patch?.body]).toEqual([`${CREDENTIALS_PATH}/openai`, { model: 'gpt-typed-1' }])
    })
})

describe('설정 AI 사용량과 설정', () => {
    const usageRoutes = (summary = usageSummary(), setting = userSetting(), stored: Credential[] = [credentialRow({ model: 'gpt-test' })]) =>
        ({
            [`GET ${USAGE_PATH}`]: () => jsonResponse(summary),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(setting),
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse(stored),
        }) satisfies FetchRoutes

    test('오늘 호출과 이번 달 토큰을 한도와 함께 보여 준다', async () => {
        useRoutes(usageRoutes())
        await renderWithQueryClient(<AiUsagePanel />)

        expect(await page.findByText('12 / 30 회')).toBeTruthy()
        expect(page.getByLabelText('오늘 호출 사용률').getAttribute('aria-valuenow')).toBe('40')
        expect(page.getByText('입력 3,000 · 출력 800 토큰(캐시 500)')).toBeTruthy()
        expect(page.getByText('15,500 토큰 · 제한 없음')).toBeTruthy()
        expect(page.getByText('입력 12,000 · 출력 3,500 토큰(캐시 1,000)')).toBeTruthy()
        expect(page.getByText('동시 실행 한도 1건')).toBeTruthy()
        expect(page.getByText(/UTC 자정\(한국 시간 오전 9시\)/)).toBeTruthy()
    })

    test('월 토큰 상한이 있으면 사용률을 그리고 상한에 닿으면 경고 색을 쓴다', async () => {
        useRoutes(usageRoutes(usageSummary({ monthly_token_limit: 15_000 })))
        await renderWithQueryClient(<AiUsagePanel />)

        const reached = await page.findByText('15,500 / 15,000 토큰')
        expect(reached.className).toContain('text-warning')
        expect(page.getByLabelText('이번 달 토큰 사용률').getAttribute('aria-valuenow')).toBe('100')
    })

    test('저장 버튼은 바뀐 값이 있고 유효할 때만 켜진다', async () => {
        useRoutes(usageRoutes())
        await renderWithQueryClient(<AiUsagePanel />)

        const limit = (await page.findByLabelText('월 토큰 상한')) as HTMLInputElement
        const save = page.getByRole('button', { name: '저장' }) as HTMLButtonElement
        expect(save.disabled).toBe(true)

        await changeValue(limit, '12a')
        expect(await page.findByText('1 이상 1,000,000,000,000 이하의 정수를 입력하세요.')).toBeTruthy()
        expect(save.disabled).toBe(true)

        await changeValue(limit, '5,000')
        expect(save.disabled).toBe(false)
        expect(page.queryByText(/이하의 정수를 입력하세요/)).toBeNull()
    })

    test('월 상한을 저장하면 PUT 으로 두 값을 함께 보내고 성공을 알린다', async () => {
        let setting = userSetting()
        const calls = useRoutes({
            ...usageRoutes(),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(setting),
            [`PUT ${SETTINGS_PATH}`]: (call) => {
                setting = { ...(call.body as Omit<typeof setting, 'updated_at'>), updated_at: NOW }
                return jsonResponse(setting)
            },
        })
        await renderWithQueryClient(<AiUsagePanel />)

        const limit = (await page.findByLabelText('월 토큰 상한')) as HTMLInputElement
        await changeValue(limit, '5000')
        await click(page.getByRole('button', { name: '저장' }))

        await waitFor(() => expect(toastTexts()).toContain('AI 설정을 저장했습니다.'))
        expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({ monthly_token_limit: 5000, default_provider: null })
        await waitFor(() => expect((page.getByLabelText('월 토큰 상한') as HTMLInputElement).value).toBe('5000'))
        expect((page.getByRole('button', { name: '저장' }) as HTMLButtonElement).disabled).toBe(true)
    })

    test('입력란을 비우고 저장하면 월 상한을 null 로 보낸다', async () => {
        const calls = useRoutes({
            ...usageRoutes(usageSummary({ monthly_token_limit: 9000 }), userSetting({ monthly_token_limit: 9000 })),
            [`PUT ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
        })
        await renderWithQueryClient(<AiUsagePanel />)

        const limit = (await page.findByLabelText('월 토큰 상한')) as HTMLInputElement
        expect(limit.value).toBe('9000')
        await changeValue(limit, '')
        await click(page.getByRole('button', { name: '저장' }))

        await waitFor(() => expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({ monthly_token_limit: null, default_provider: null }))
    })

    test('기본 공급자는 등록한 키 중에서 고르고 함께 저장한다', async () => {
        const calls = useRoutes({
            ...usageRoutes(usageSummary(), userSetting(), [
                credentialRow({ provider: 'openai', model: 'gpt-test' }),
                credentialRow({ provider: 'ollama' }),
            ]),
            [`PUT ${SETTINGS_PATH}`]: () => jsonResponse(userSetting({ default_provider: 'ollama' })),
        })
        await renderWithQueryClient(<AiUsagePanel />)

        await openSelect(await page.findByRole('combobox', { name: '기본 공급자' }))
        expect(page.queryByRole('option', { name: 'Anthropic (Claude)' })).toBeNull()
        expect(await page.findByRole('option', { name: '자동 (가장 먼저 등록한 키)' })).toBeTruthy()
        await click(page.getByRole('option', { name: 'Ollama Cloud' }))
        await click(page.getByRole('button', { name: '저장' }))

        await waitFor(() =>
            expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({ monthly_token_limit: null, default_provider: 'ollama' }),
        )
    })

    test('사용량 요청이 실패하면 오류 상태를 보이고 사용량 표시는 그리지 않는다', async () => {
        useRoutes({
            [`GET ${USAGE_PATH}`]: () => errorResponse(500, 'INTERNAL_ERROR', '서버 오류'),
            [`GET ${SETTINGS_PATH}`]: () => jsonResponse(userSetting()),
            [`GET ${CREDENTIALS_PATH}`]: () => jsonResponse([]),
        })
        await renderWithQueryClient(<AiUsagePanel />)

        expect(await page.findByText('AI 사용량을 불러오지 못했습니다')).toBeTruthy()
        expect(page.queryByText(/동시 실행 한도/)).toBeNull()
        expect(page.queryByLabelText('오늘 호출 사용률')).toBeNull()
    })
})
