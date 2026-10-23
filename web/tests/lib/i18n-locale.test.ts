import { describe, expect, test } from 'bun:test'
import { buildLocaleCookie, DEFAULT_LOCALE, isLocale, LOCALE_COOKIE_NAME, negotiateLocale, resolveRequestLocale } from '@shared/i18n/locale'

describe('negotiateLocale', () => {
    test('품질 값이 높은 지원 언어를 고른다', () => {
        expect(negotiateLocale('fr-FR,fr;q=0.9,ja;q=0.8,en;q=0.7')).toBe('ja')
        expect(negotiateLocale('en;q=0.5,ja;q=0.9')).toBe('ja')
    })

    test('지역 꼬리표가 붙어도 첫 부분으로 맞춘다', () => {
        expect(negotiateLocale('en-US,en;q=0.9')).toBe('en')
        expect(negotiateLocale('KO-kr')).toBe('ko')
    })

    test('품질 값이 같으면 헤더 순서를 따른다', () => {
        expect(negotiateLocale('ja;q=0.8,en;q=0.8')).toBe('ja')
    })

    test('지원 언어가 없거나 헤더가 없으면 null 이다', () => {
        expect(negotiateLocale('fr-FR,de;q=0.9,*;q=0.1')).toBeNull()
        expect(negotiateLocale(null)).toBeNull()
        expect(negotiateLocale('')).toBeNull()
    })

    test('q=0 과 잘못된 품질 값은 건너뛴다', () => {
        expect(negotiateLocale('en;q=0,ja')).toBe('ja')
        expect(negotiateLocale('en;q=abc,ko;q=0.2')).toBe('ko')
    })
})

describe('resolveRequestLocale', () => {
    test('올바른 쿠키가 헤더보다 앞선다', () => {
        expect(resolveRequestLocale({ cookieLocale: 'ja', acceptLanguage: 'en-US' })).toBe('ja')
    })

    test('쿠키가 없거나 지원 밖이면 Accept-Language 로 정한다', () => {
        expect(resolveRequestLocale({ cookieLocale: undefined, acceptLanguage: 'en-US,en;q=0.9' })).toBe('en')
        expect(resolveRequestLocale({ cookieLocale: 'fr', acceptLanguage: 'ja' })).toBe('ja')
    })

    test('쿠키도 헤더도 없는 봇은 기본 한국어다', () => {
        expect(resolveRequestLocale({ cookieLocale: undefined, acceptLanguage: null })).toBe(DEFAULT_LOCALE)
        expect(DEFAULT_LOCALE).toBe('ko')
    })
})

describe('isLocale', () => {
    test('지원하는 세 언어만 받는다', () => {
        expect(['ko', 'en', 'ja'].every(isLocale)).toBe(true)
        expect(isLocale('en-US')).toBe(false)
        expect(isLocale(undefined)).toBe(false)
    })
})

describe('buildLocaleCookie', () => {
    test('사이트 전체 경로에 1년짜리 Lax 쿠키를 만든다', () => {
        expect(buildLocaleCookie('en')).toBe(`${LOCALE_COOKIE_NAME}=en; Path=/; Max-Age=31536000; SameSite=Lax`)
    })
})
