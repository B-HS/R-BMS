import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { changeValue, click, page, waitFor } = await import('../support/dom-harness')
const { toast } = await import('sonner')
const { AiLimitsPanel } = await import('../../src/widgets/admin/ai-limits-panel')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient, toastTexts } = await import('../support/ai-screen')
const { withLocale } = await import('../support/browse-intl')

type Limits = { daily_calls: number; concurrent_runs: number; default_monthly_token_limit: number | null; updated_at: number | null }

const LIMITS_PATH = '/api/admin/ai/limits'
const NOW = 1_780_000_000_000
const DEFAULT_LIMITS: Limits = { daily_calls: 30, concurrent_runs: 1, default_monthly_token_limit: null, updated_at: null }

let restoreFetch: () => void = () => undefined

const useLimitRoutes = (initial: Limits = DEFAULT_LIMITS, onPut?: () => Response) => {
    let stored = initial
    const installed = installFetchRoutes({
        [`GET ${LIMITS_PATH}`]: () => jsonResponse(stored),
        [`PUT ${LIMITS_PATH}`]: (call) => {
            if (onPut) return onPut()
            stored = { ...(call.body as Omit<Limits, 'updated_at'>), updated_at: NOW }
            return jsonResponse(stored)
        },
    })
    restoreFetch = installed.restore
    return installed.calls
}

const field = (name: string) => page.getByLabelText(name) as HTMLInputElement
const saveButton = () => page.getByRole('button', { name: '저장' }) as HTMLButtonElement

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
    toast.dismiss()
})

describe('운영자 AI 한도 패널', () => {
    test('저장된 한도를 채워 보이고 바꾸기 전에는 저장 버튼이 꺼져 있다', async () => {
        useLimitRoutes()
        await renderWithQueryClient(withLocale(<AiLimitsPanel />))

        expect(((await page.findByLabelText('사용자당 하루 호출 수')) as HTMLInputElement).value).toBe('30')
        expect(field('사용자당 동시 실행 수').value).toBe('1')
        expect(field('기본 월 토큰 상한').value).toBe('')
        expect(page.getByText('아직 바꾼 적이 없어 기본값입니다.')).toBeTruthy()
        expect(saveButton().disabled).toBe(true)
    })

    test('범위를 벗어난 값은 필드별 오류를 보이고 저장하지 못하게 한다', async () => {
        useLimitRoutes()
        await renderWithQueryClient(withLocale(<AiLimitsPanel />))
        await page.findByLabelText('사용자당 하루 호출 수')

        for (const value of ['0', '1001', 'abc', '']) {
            await changeValue(field('사용자당 하루 호출 수'), value)
            expect(page.getByText('1 이상 1,000 이하의 정수를 입력하세요.')).toBeTruthy()
            expect(saveButton().disabled).toBe(true)
        }
        await changeValue(field('사용자당 하루 호출 수'), '40')
        await changeValue(field('사용자당 동시 실행 수'), '6')
        expect(page.getByText('1 이상 5 이하의 정수를 입력하세요.')).toBeTruthy()
        expect(saveButton().disabled).toBe(true)
        await changeValue(field('사용자당 동시 실행 수'), '2')
        await changeValue(field('기본 월 토큰 상한'), '-5')
        expect(page.getByText('1 이상 1,000,000,000,000 이하의 정수를 입력하세요.')).toBeTruthy()
        expect(saveButton().disabled).toBe(true)
    })

    test('유효한 값을 저장하면 PUT 으로 세 값을 보내고 저장된 값과 변경 시각을 보인다', async () => {
        const calls = useLimitRoutes()
        await renderWithQueryClient(withLocale(<AiLimitsPanel />))
        await page.findByLabelText('사용자당 하루 호출 수')

        await changeValue(field('사용자당 하루 호출 수'), '50')
        await changeValue(field('사용자당 동시 실행 수'), '2')
        await changeValue(field('기본 월 토큰 상한'), '2,000,000')
        await click(saveButton())

        await waitFor(() => expect(toastTexts()).toContain('AI 사용 한도를 저장했습니다.'))
        expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({
            daily_calls: 50,
            concurrent_runs: 2,
            default_monthly_token_limit: 2_000_000,
        })
        await waitFor(() => expect(field('기본 월 토큰 상한').value).toBe('2000000'))
        expect(page.queryByText('아직 바꾼 적이 없어 기본값입니다.')).toBeNull()
        expect(page.getByText(/마지막 변경 20/)).toBeTruthy()
        expect(saveButton().disabled).toBe(true)
    })

    test('월 토큰 상한을 비우면 제한 없음(null)으로 저장한다', async () => {
        const calls = useLimitRoutes({ daily_calls: 30, concurrent_runs: 1, default_monthly_token_limit: 9000, updated_at: NOW })
        await renderWithQueryClient(withLocale(<AiLimitsPanel />))
        await page.findByLabelText('사용자당 하루 호출 수')

        await changeValue(field('기본 월 토큰 상한'), '')
        await click(saveButton())

        await waitFor(() =>
            expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({
                daily_calls: 30,
                concurrent_runs: 1,
                default_monthly_token_limit: null,
            }),
        )
    })

    test('관리자가 아니라는 응답이면 서버 메시지를 알리고 입력값을 그대로 둔다', async () => {
        useLimitRoutes(DEFAULT_LIMITS, () => errorResponse(403, 'FORBIDDEN', '접근 권한이 없습니다.'))
        await renderWithQueryClient(withLocale(<AiLimitsPanel />))
        await page.findByLabelText('사용자당 하루 호출 수')

        await changeValue(field('사용자당 하루 호출 수'), '45')
        await click(saveButton())

        await waitFor(() => expect(toastTexts()).toContain('접근 권한이 없습니다.'))
        expect(field('사용자당 하루 호출 수').value).toBe('45')
    })

    test('불러오기에 실패하면 오류 상태를 보이고 폼은 그리지 않는다', async () => {
        installFetchRoutes({}).restore()
        const installed = installFetchRoutes({ [`GET ${LIMITS_PATH}`]: () => errorResponse(403, 'FORBIDDEN', '접근 권한이 없습니다.') })
        restoreFetch = installed.restore
        await renderWithQueryClient(withLocale(<AiLimitsPanel />))

        expect(await page.findByText('AI 사용 한도를 불러오지 못했습니다')).toBeTruthy()
        expect(page.queryByLabelText('사용자당 하루 호출 수')).toBeNull()
    })
})
