import enAi from '../../../messages/en/ai.json'
import enBrowse from '../../../messages/en/browse.json'
import enCommon from '../../../messages/en/common.json'
import enPages from '../../../messages/en/pages.json'
import enServer from '../../../messages/en/server.json'
import enSettings from '../../../messages/en/settings.json'
import jaAi from '../../../messages/ja/ai.json'
import jaBrowse from '../../../messages/ja/browse.json'
import jaCommon from '../../../messages/ja/common.json'
import jaPages from '../../../messages/ja/pages.json'
import jaServer from '../../../messages/ja/server.json'
import jaSettings from '../../../messages/ja/settings.json'
import koAi from '../../../messages/ko/ai.json'
import koBrowse from '../../../messages/ko/browse.json'
import koCommon from '../../../messages/ko/common.json'
import koPages from '../../../messages/ko/pages.json'
import koServer from '../../../messages/ko/server.json'
import koSettings from '../../../messages/ko/settings.json'
import type { Locale } from '@shared/i18n/locale'

export const MESSAGE_NAMESPACES = ['common', 'ai', 'settings', 'browse', 'pages', 'server'] as const

const KO_MESSAGES = { common: koCommon, ai: koAi, settings: koSettings, browse: koBrowse, pages: koPages, server: koServer } satisfies Record<
    (typeof MESSAGE_NAMESPACES)[number],
    object
>

/** Korean is the source catalog: every other locale must carry the same keys. */
export type Messages = typeof KO_MESSAGES

export const MESSAGES: Record<Locale, Messages> = {
    ko: KO_MESSAGES,
    en: { common: enCommon, ai: enAi, settings: enSettings, browse: enBrowse, pages: enPages, server: enServer },
    ja: { common: jaCommon, ai: jaAi, settings: jaSettings, browse: jaBrowse, pages: jaPages, server: jaServer },
}
