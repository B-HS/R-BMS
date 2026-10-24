import { afterAll, afterEach, describe, expect, mock, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

mock.module('@shared/lib/auth-client', () => ({
    authClient: { useSession: () => ({ data: null, isPending: false }), $store: { notify: () => undefined } },
}))

const { mount, page, submit, unmountAll } = await import('../support/dom-harness')
const { LoginForm } = await import('../../src/widgets/auth/login-form')
const { SignupForm } = await import('../../src/widgets/auth/signup-form')
const { withLocale } = await import('../support/browse-intl')

type Locale = import('../../src/shared/i18n/locale').Locale

const LOCALES = ['ko', 'en', 'ja'] as const satisfies readonly Locale[]

const router = { push: () => undefined, replace: () => undefined, back: () => undefined, forward: () => undefined, refresh: () => undefined }

const LOGIN_ERRORS = {
    ko: ['아이디를 입력하세요', '비밀번호는 8자 이상이어야 합니다'],
    en: ['Enter your ID', 'The password must be at least 8 characters'],
    ja: ['IDを入力してください', 'パスワードは8文字以上で入力してください'],
} as const

const SIGNUP_ERRORS = {
    ko: ['아이디는 3자 이상이어야 합니다', '이메일 형식이 올바르지 않습니다', '표시 이름을 입력하세요', '비밀번호는 8자 이상이어야 합니다'],
    en: [
        'The ID must be at least 3 characters',
        'The email address is not valid',
        'Enter a display name',
        'The password must be at least 8 characters',
    ],
    ja: [
        'IDは3文字以上で入力してください',
        'メールアドレスの形式が正しくありません',
        '表示名を入力してください',
        'パスワードは8文字以上で入力してください',
    ],
} as const

const mountForm = (locale: Locale, form: React.ReactNode) =>
    mount(
        <AppRouterContext.Provider value={router as never}>
            <QueryClientProvider client={new QueryClient()}>{withLocale(form, locale)}</QueryClientProvider>
        </AppRouterContext.Provider>,
    )

const submitOnlyForm = () => submit(document.querySelector('form') as HTMLFormElement)

afterEach(() => unmountAll())

describe('인증 폼의 검증 문구', () => {
    for (const locale of LOCALES) {
        test(`${locale}: 로그인 폼은 빈 값 제출에 그 언어의 오류를 보인다`, async () => {
            await mountForm(locale, <LoginForm />)
            await submitOnlyForm()
            for (const message of LOGIN_ERRORS[locale]) expect(await page.findByText(message)).toBeTruthy()
        })

        test(`${locale}: 가입 폼은 빈 값 제출에 필드별 오류를 그 언어로 보인다`, async () => {
            await mountForm(locale, <SignupForm />)
            await submitOnlyForm()
            for (const message of SIGNUP_ERRORS[locale]) expect(await page.findByText(message)).toBeTruthy()
        })
    }
})
