import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost/' })

afterAll(() => GlobalRegistrator.unregister())

const { runInAct } = await import('../support/dom-harness')
const { createRoot } = await import('react-dom/client')
const { LOCALE_COOKIE_NAME } = await import('../../src/shared/i18n/locale')
const { default: GlobalErrorPage } = await import('../../src/app/global-error')

const cases: [string, string, string, string][] = [
    ['ko', '문제가 생겼습니다', '다시 시도', '오류 | R-BMS'],
    ['en', 'Something went wrong', 'Try again', 'Error | R-BMS'],
    ['ja', '問題が発生しました', 'もう一度試す', 'エラー | R-BMS'],
]

const renderGlobalError = async () => {
    const root = createRoot(document)
    await runInAct(() => root.render(<GlobalErrorPage error={new Error('boom')} retry={() => undefined} />))
    return root
}

afterEach(() => {
    document.cookie = `${LOCALE_COOKIE_NAME}=; Path=/; Max-Age=0`
})

describe('global-error 는 루트 레이아웃 밖이라 NEXT_LOCALE 쿠키로 언어를 고른다', () => {
    test.each(cases)('%s 쿠키면 그 언어로 제목과 버튼과 문서 언어를 쓴다', async (locale, heading, retry, title) => {
        document.cookie = `${LOCALE_COOKIE_NAME}=${locale}; Path=/`
        const root = await renderGlobalError()

        expect(document.documentElement.lang).toBe(locale)
        expect(document.querySelector('h1')?.textContent).toBe(heading)
        expect(document.querySelector('button')?.textContent).toBe(retry)
        expect(document.title).toBe(title)
        await runInAct(() => root.unmount())
    })

    const browserCases: [string[], string][] = [
        [['ja-JP', 'en'], 'ja'],
        [['fr-FR', 'de'], 'ko'],
    ]

    test.each(browserCases)('쿠키가 없으면 브라우저 언어 %j 로 %s 를 고른다', async (languages, locale) => {
        Object.defineProperty(navigator, 'languages', { value: languages, configurable: true })
        const root = await renderGlobalError()

        expect(document.documentElement.lang).toBe(locale)
        await runInAct(() => root.unmount())
    })
})
