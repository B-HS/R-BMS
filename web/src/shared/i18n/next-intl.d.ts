import type { Locale } from '@shared/i18n/locale'
import type { Messages } from '@shared/i18n/messages'

declare module 'next-intl' {
    interface AppConfig {
        Locale: Locale
        Messages: Messages
    }
}
