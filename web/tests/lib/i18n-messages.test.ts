import { describe, expect, test } from 'bun:test'
import { readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { DEFAULT_LOCALE, LOCALES } from '@shared/i18n/locale'
import { MESSAGE_NAMESPACES, MESSAGES } from '@shared/i18n/messages'

const MESSAGES_DIRECTORY = fileURLToPath(new URL('../../messages', import.meta.url))
const JSON_EXTENSION = '.json'
const ICU_ARGUMENT = /\{\s*([A-Za-z0-9_]+)/g

const flatten = (value: unknown, path: string): [string, unknown][] =>
    typeof value === 'object' && value !== null && !Array.isArray(value)
        ? Object.entries(value).flatMap(([key, child]) => flatten(child, path ? `${path}.${key}` : key))
        : [[path, value]]

const argumentsOf = (message: string) => [...new Set([...message.matchAll(ICU_ARGUMENT)].map((match) => match[1]))].toSorted()

describe('웹 메시지 카탈로그', () => {
    test('언어마다 정해진 네임스페이스 파일만 있다', () => {
        expect(readdirSync(MESSAGES_DIRECTORY).toSorted()).toEqual([...LOCALES].toSorted())
        for (const locale of LOCALES) {
            const files = readdirSync(`${MESSAGES_DIRECTORY}/${locale}`).toSorted()
            expect(files).toEqual(MESSAGE_NAMESPACES.map((namespace) => `${namespace}${JSON_EXTENSION}`).toSorted())
        }
    })

    test('모든 언어가 한국어와 같은 키를 갖고 값은 비지 않은 문자열이다', () => {
        const sourceEntries = flatten(MESSAGES[DEFAULT_LOCALE], '')
        for (const locale of LOCALES) {
            const entries = flatten(MESSAGES[locale], '')
            expect(entries.map(([key]) => key).toSorted()).toEqual(sourceEntries.map(([key]) => key).toSorted())
            for (const [key, message] of entries) {
                expect(typeof message, `${locale}:${key}`).toBe('string')
                expect(String(message).trim().length, `${locale}:${key}`).toBeGreaterThan(0)
            }
        }
    })

    test('모든 언어가 한국어와 같은 자리표를 쓴다', () => {
        const source = new Map(flatten(MESSAGES[DEFAULT_LOCALE], '').map(([key, message]) => [key, argumentsOf(String(message))]))
        for (const locale of LOCALES) {
            for (const [key, message] of flatten(MESSAGES[locale], '')) {
                expect(argumentsOf(String(message)), `${locale}:${key}`).toEqual(source.get(key) ?? [])
            }
        }
    })
})
