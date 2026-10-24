import type { ReactNode } from 'react'
import { NextIntlClientProvider } from 'next-intl'
import { DEFAULT_LOCALE, type Locale } from '../../src/shared/i18n/locale'
import { MESSAGES } from '../../src/shared/i18n/messages'

const TEST_TIME_ZONE = 'UTC'

/** Wraps a component under test in the message provider the root layout supplies in the app. */
export const withIntl = (ui: ReactNode, locale: Locale = DEFAULT_LOCALE) => (
    <NextIntlClientProvider locale={locale} messages={MESSAGES[locale]} timeZone={TEST_TIME_ZONE}>
        {ui}
    </NextIntlClientProvider>
)
