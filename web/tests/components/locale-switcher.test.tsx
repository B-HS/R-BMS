import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { AppRouterContext, type AppRouterInstance } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { DISPLAY_TIME_ZONE } from '../../src/shared/lib/format'

GlobalRegistrator.register({ url: 'http://localhost/' })

afterAll(() => GlobalRegistrator.unregister())

const refreshCalls: string[] = []
const ignoreNavigation = () => undefined
const ROUTER: AppRouterInstance = {
    back: ignoreNavigation,
    forward: ignoreNavigation,
    push: ignoreNavigation,
    replace: ignoreNavigation,
    prefetch: ignoreNavigation,
    refresh: () => void refreshCalls.push('refresh'),
    bfcacheId: 'locale-switcher-test',
}

const { click, keyDown, page, unmountAll, mount } = await import('../support/dom-harness')
const { NextIntlClientProvider } = await import('next-intl')
const { LOCALE_COOKIE_NAME } = await import('../../src/shared/i18n/locale')
const { MESSAGES } = await import('../../src/shared/i18n/messages')
const { SidebarProvider } = await import('../../src/shared/ui/sidebar')
const { TooltipProvider } = await import('../../src/shared/ui/tooltip')
const { LocaleSwitcher } = await import('../../src/widgets/locale-switcher/locale-switcher')

type Locale = import('../../src/shared/i18n/locale').Locale

const renderSwitcher = (locale: Locale) =>
    mount(
        <AppRouterContext.Provider value={ROUTER}>
            <NextIntlClientProvider locale={locale} messages={MESSAGES[locale]} timeZone={DISPLAY_TIME_ZONE}>
                <TooltipProvider>
                    <SidebarProvider>
                        <LocaleSwitcher />
                    </SidebarProvider>
                </TooltipProvider>
            </NextIntlClientProvider>
        </AppRouterContext.Provider>,
    )

const localeCookie = () =>
    document.cookie
        .split('; ')
        .find((entry) => entry.startsWith(`${LOCALE_COOKIE_NAME}=`))
        ?.slice(LOCALE_COOKIE_NAME.length + 1)

afterEach(async () => {
    await unmountAll()
    refreshCalls.length = 0
    document.cookie = `${LOCALE_COOKIE_NAME}=; Path=/; Max-Age=0`
})

describe('LocaleSwitcher', () => {
    test('현재 언어로 이름을 붙이고 세 언어를 각자의 이름으로 보여 준다', async () => {
        await renderSwitcher('ja')
        await keyDown(page.getByRole('button', { name: '言語' }), 'Enter')

        const options = await page.findByRole('menuitemradio', { name: '日本語' })
        expect(options.getAttribute('aria-checked')).toBe('true')
        expect(page.getAllByRole('menuitemradio').map((option) => option.textContent)).toEqual(['한국어', 'English', '日本語'])
    })

    test('다른 언어를 고르면 쿠키를 쓰고 화면을 새로 받는다', async () => {
        await renderSwitcher('ko')
        await keyDown(page.getByRole('button', { name: '언어' }), 'Enter')
        await click(await page.findByRole('menuitemradio', { name: 'English' }))

        expect(localeCookie()).toBe('en')
        expect(refreshCalls).toEqual(['refresh'])
    })

    test('지금 언어를 다시 고르면 아무것도 하지 않는다', async () => {
        await renderSwitcher('ko')
        await keyDown(page.getByRole('button', { name: '언어' }), 'Enter')
        await click(await page.findByRole('menuitemradio', { name: '한국어' }))

        expect(localeCookie()).toBeUndefined()
        expect(refreshCalls).toEqual([])
    })
})
