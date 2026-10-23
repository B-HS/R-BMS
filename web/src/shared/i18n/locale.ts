import { hasLocale } from 'next-intl'

export const LOCALES = ['ko', 'en', 'ja'] as const
export type Locale = (typeof LOCALES)[number]

export const DEFAULT_LOCALE: Locale = 'ko'
export const LOCALE_COOKIE_NAME = 'NEXT_LOCALE'

const SECONDS_PER_DAY = 86_400
const LOCALE_COOKIE_MAX_AGE_DAYS = 365
const LOCALE_COOKIE_MAX_AGE_SECONDS = SECONDS_PER_DAY * LOCALE_COOKIE_MAX_AGE_DAYS

export const LOCALE_NATIVE_NAME: Record<Locale, string> = { ko: '한국어', en: 'English', ja: '日本語' }

const FULL_QUALITY = 1
const QUALITY_PREFIX = 'q='

export const isLocale = (value: unknown): value is Locale => hasLocale(LOCALES, value)

/**
 * Picks the first supported locale from an `Accept-Language` header, honouring quality values and matching on the primary subtag
 * (`en-US` matches `en`). Returns `null` when the header names no supported language.
 */
export const negotiateLocale = (acceptLanguage: string | null | undefined) =>
    (acceptLanguage ?? '')
        .split(',')
        .map((entry) => {
            const [tag = '', ...parameters] = entry.split(';').map((part) => part.trim())
            const quality = parameters.find((parameter) => parameter.startsWith(QUALITY_PREFIX))
            return {
                language: tag.toLowerCase().split('-')[0],
                quality: quality === undefined ? FULL_QUALITY : Number(quality.slice(QUALITY_PREFIX.length)),
            }
        })
        .filter(({ quality }) => Number.isFinite(quality) && quality > 0)
        .toSorted((left, right) => right.quality - left.quality)
        .map(({ language }) => language)
        .find(isLocale) ?? null

/** The locale cookie wins; without a valid one the browser's languages decide, and anything else falls back to Korean. */
export const resolveRequestLocale = ({ cookieLocale, acceptLanguage }: { cookieLocale: string | undefined; acceptLanguage: string | null }) =>
    isLocale(cookieLocale) ? cookieLocale : (negotiateLocale(acceptLanguage) ?? DEFAULT_LOCALE)

export const buildLocaleCookie = (locale: Locale) => `${LOCALE_COOKIE_NAME}=${locale}; Path=/; Max-Age=${LOCALE_COOKIE_MAX_AGE_SECONDS}; SameSite=Lax`
