import { describe, expect, test } from 'bun:test'
import { readListParam, readNumberParam, readParam, serializeSearchParams, toggleListValue } from '../../src/shared/lib/search-params'

describe('URL 직렬화', () => {
    test('빈 값과 빈 배열은 쿼리에서 제거된다', () => {
        expect(serializeSearchParams({ q: 'conflict', mode: '', level: undefined, lamp: [] })).toBe('q=conflict')
    })

    test('배열은 콤마로 합쳐진다', () => {
        expect(serializeSearchParams({ lamp: ['Hard', 'ExHard'], page: 2 })).toBe('lamp=Hard%2CExHard&page=2')
    })

    test('toggleListValue 는 없으면 추가하고 있으면 제거한다', () => {
        expect(toggleListValue(['Hard'], 'ExHard')).toEqual(['Hard', 'ExHard'])
        expect(toggleListValue(['Hard', 'ExHard'], 'Hard')).toEqual(['ExHard'])
    })

    test('readParam 은 배열 파라미터의 첫 값을 쓴다', () => {
        expect(readParam({ q: ['a', 'b'] }, 'q')).toBe('a')
        expect(readParam({}, 'q')).toBeUndefined()
    })

    test('readNumberParam 은 숫자가 아니면 기본값을 쓴다', () => {
        expect(readNumberParam({ page: '3' }, 'page', 1)).toBe(3)
        expect(readNumberParam({ page: 'abc' }, 'page', 1)).toBe(1)
        expect(readNumberParam({}, 'page', 1)).toBe(1)
    })

    test('readListParam 은 콤마 문자열을 배열로 되돌린다', () => {
        expect(readListParam({ lamp: 'Hard,ExHard' }, 'lamp')).toEqual(['Hard', 'ExHard'])
        expect(readListParam({}, 'lamp')).toEqual([])
    })
})
