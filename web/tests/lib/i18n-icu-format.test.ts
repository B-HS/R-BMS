import { describe, expect, test } from 'bun:test'
import { createTranslator } from 'next-intl'
import { LOCALES } from '@shared/i18n/locale'
import { MESSAGES } from '@shared/i18n/messages'

const ICU_ARGUMENT = /\{\s*([A-Za-z0-9_]+)/g
const RICH_TAG = /<([A-Za-z][A-Za-z0-9]*)>/g
const DOUBLED_APOSTROPHE = /''/g
const QUOTED_SPAN = /'[^']*'/g
const SINGULAR_SAMPLE = 1
const PLURAL_SAMPLE = 1234

const flatten = (value: unknown, path: string): [string, string][] =>
    typeof value === 'object' && value !== null && !Array.isArray(value)
        ? Object.entries(value).flatMap(([key, child]) => flatten(child, path ? `${path}.${key}` : key))
        : [[path, String(value)]]

const argumentNames = (message: string) => [...new Set([...message.matchAll(ICU_ARGUMENT)].map((match) => match[1]))]

const tagNames = (message: string) => [
    ...new Set([...message.replace(DOUBLED_APOSTROPHE, '').replace(QUOTED_SPAN, '').matchAll(RICH_TAG)].map((match) => match[1])),
]

const keepChunks = (chunks: string) => chunks

describe('웹 메시지의 ICU 문법', () => {
    for (const locale of LOCALES) {
        test(`${locale}: 모든 메시지가 오류 없이 단수·복수 값으로 포맷된다`, () => {
            const failures: string[] = []
            for (const [key, message] of flatten(MESSAGES[locale], '')) {
                for (const sample of [SINGULAR_SAMPLE, PLURAL_SAMPLE]) {
                    const translator = createTranslator({
                        locale,
                        timeZone: 'UTC',
                        messages: { entry: message },
                        onError: (error) => void failures.push(`${locale}:${key}:${error.code}`),
                    })
                    const values = Object.fromEntries([
                        ...argumentNames(message).map((name) => [name, sample] as const),
                        ...tagNames(message).map((name) => [name, keepChunks] as const),
                    ])
                    const rendered = translator.markup('entry', values)
                    if (/[{}]/.test(rendered)) failures.push(`${locale}:${key}:leftover brace in ${JSON.stringify(rendered)}`)
                    if (rendered.trim() === '') failures.push(`${locale}:${key}:empty`)
                }
            }
            expect(failures).toEqual([])
        })
    }

    test('작은따옴표로 감싼 꺾쇠는 태그가 아니라 글자 그대로 나온다', () => {
        const translator = createTranslator({ locale: 'ko', timeZone: 'UTC', messages: MESSAGES.ko, namespace: 'browse.guide' })
        expect(translator('ranked.rows.scratchAuto.description')).toContain('IR: RANK #<순위> (SCRATCH AUTO) 로 나옵니다.')
        expect(translator('apiToken.code', { url: 'https://example.test' })).toBe(
            "curl -H 'Authorization: Bearer rbms_<발급받은_토큰>' https://example.test/auth/me",
        )
    })

    test('숫자 자리표는 천 단위 구분을 쓰고 영어 복수형을 가른다', () => {
        const ko = createTranslator({ locale: 'ko', timeZone: 'UTC', messages: MESSAGES.ko, namespace: 'browse.folder' })
        const en = createTranslator({ locale: 'en', timeZone: 'UTC', messages: MESSAGES.en, namespace: 'browse.folder' })
        expect(ko('itemCount', { count: 12_345 })).toBe('12,345곡')
        expect(en('itemCount', { count: 1 })).toBe('1 chart')
        expect(en('itemCount', { count: 12_345 })).toBe('12,345 charts')
    })
})
