'use client'

import { useSyncExternalStore } from 'react'
import '@app/globals.css'
import { SITE_NAME } from '@shared/constants/site'
import { DEFAULT_LOCALE, LOCALE_COOKIE_NAME, resolveRequestLocale, type Locale } from '@shared/i18n/locale'
import enPages from '../../messages/en/pages.json'
import jaPages from '../../messages/ja/pages.json'
import koPages from '../../messages/ko/pages.json'

const GLOBAL_ERROR_MESSAGES = {
    ko: koPages.globalError,
    en: enPages.globalError,
    ja: jaPages.globalError,
} satisfies Record<Locale, typeof koPages.globalError>

const subscribeToNothing = () => () => undefined

const readLocale = () =>
    resolveRequestLocale({
        cookieLocale: document.cookie
            .split('; ')
            .find((entry) => entry.startsWith(`${LOCALE_COOKIE_NAME}=`))
            ?.slice(LOCALE_COOKIE_NAME.length + 1),
        acceptLanguage: navigator.languages.join(','),
    })

const GlobalErrorPage = ({ retry }: { error: Error & { digest?: string }; retry: () => void }) => {
    const locale = useSyncExternalStore(subscribeToNothing, readLocale, () => DEFAULT_LOCALE)
    const messages = GLOBAL_ERROR_MESSAGES[locale]

    return (
        <html lang={locale} className='dark' data-theme='dark'>
            <body className='bg-background text-foreground flex min-h-dvh flex-col items-center justify-center gap-3 p-6 text-center font-sans antialiased'>
                <title>{`${messages.title} | ${SITE_NAME}`}</title>
                <p className='rbms-micro-label'>{SITE_NAME}</p>
                <h1 className='text-sm font-semibold'>{messages.heading}</h1>
                <p className='text-muted-foreground text-xs'>{messages.description}</p>
                <button
                    type='button'
                    onClick={() => retry()}
                    className='border-border hover:bg-muted focus-visible:ring-ring/50 h-7 border px-2.5 text-xs font-medium outline-none focus-visible:ring-3'>
                    {messages.retry}
                </button>
            </body>
        </html>
    )
}

export default GlobalErrorPage
