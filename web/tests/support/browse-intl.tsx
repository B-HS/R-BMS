import type { ReactNode } from 'react'
import { NextIntlClientProvider } from 'next-intl'
import { DISPLAY_TIME_ZONE } from '../../src/shared/lib/format'
import type { Locale } from '../../src/shared/i18n/locale'
import { MESSAGES } from '../../src/shared/i18n/messages'

export const withLocale = (ui: ReactNode, locale: Locale = 'ko') => (
    <NextIntlClientProvider locale={locale} messages={MESSAGES[locale]} timeZone={DISPLAY_TIME_ZONE}>
        {ui}
    </NextIntlClientProvider>
)
