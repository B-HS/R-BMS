import { describe, expect, test } from 'bun:test'
import { Glob } from 'bun'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { buildApiUrl } from '../../src/shared/lib/fetch'

const SRC_ROOT = join(import.meta.dir, '../../src')
const SERVER_PREFETCH_CALL = /\b(prefetchQuery|fetchQuery|fetchQueryOrNotFound)\(/g
const SERVER_SOURCE_GLOBS = ['widgets/**/*.{ts,tsx}', 'app/**/*.{ts,tsx}']

const readBalanced = (source: string, openIndex: number) => {
    let depth = 0
    for (let index = openIndex; index < source.length; index += 1) {
        if (source[index] === '(') depth += 1
        if (source[index] === ')') depth -= 1
        if (depth === 0) return source.slice(openIndex, index + 1)
    }
    return source.slice(openIndex)
}

const sourceFiles = SERVER_SOURCE_GLOBS.flatMap((pattern) => [...new Glob(pattern).scanSync({ cwd: SRC_ROOT })])

describe('서버 컴포넌트는 자기 공개 주소로 HTTP 요청을 보내지 않는다', () => {
    test('buildApiUrl 은 서버에서도 origin 을 붙이지 않아 서버 쪽 자기 호출이 URL 오류로 드러난다', () => {
        expect(buildApiUrl('/fe/stats/summary')).toBe('/api/fe/stats/summary')
        expect(buildApiUrl('/fe/charts/search', { q: 'a', page: 2 })).toBe('/api/fe/charts/search?q=a&page=2')
    })

    test('서버에서 프리페치하는 모든 호출이 queryFn 을 서버 직접 읽기로 바꿔 끼운다', () => {
        const calls = sourceFiles.flatMap((file) => {
            const source = readFileSync(join(SRC_ROOT, file), 'utf8')
            return [...source.matchAll(SERVER_PREFETCH_CALL)].map((match) => ({
                file,
                call: readBalanced(source, (match.index ?? 0) + match[0].length - 1),
            }))
        })
        expect(calls.length).toBeGreaterThanOrEqual(10)
        expect(calls.filter(({ call }) => !call.includes('queryFn')).map(({ file }) => file)).toEqual([])
    })

    test('서버 파일이 NEXT_PUBLIC_APP_URL 이나 VERCEL_URL 로 자기 주소를 만들지 않는다', () => {
        const offenders = sourceFiles.filter((file) => /NEXT_PUBLIC_APP_URL|VERCEL_URL/.test(readFileSync(join(SRC_ROOT, file), 'utf8')))
        expect(offenders).toEqual([])
    })
})
