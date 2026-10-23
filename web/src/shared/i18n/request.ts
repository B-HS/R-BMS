import 'server-only'
import { cookies, headers } from 'next/headers'
import { getRequestConfig } from 'next-intl/server'
import { LOCALE_COOKIE_NAME, resolveRequestLocale } from '@shared/i18n/locale'
import { MESSAGES } from '@shared/i18n/messages'

export default getRequestConfig(async () => {
    const [cookieStore, headerStore] = await Promise.all([cookies(), headers()])
    const locale = resolveRequestLocale({
        cookieLocale: cookieStore.get(LOCALE_COOKIE_NAME)?.value,
        acceptLanguage: headerStore.get('accept-language'),
    })
    return { locale, messages: MESSAGES[locale] }
})
