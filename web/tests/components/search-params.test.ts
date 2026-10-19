import { describe, expect, test } from 'bun:test'
import {
    readBooleanParam,
    readListParam,
    readNumberParam,
    readParam,
    serializeSearchParams,
    toggleListValue,
    writeBooleanParam,
} from '../../src/shared/lib/search-params'

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

    test('readBooleanParam 은 true 와 false 를 읽고 그 밖의 값과 없는 값은 기본값을 쓴다', () => {
        expect(readBooleanParam({ flag: 'true' }, 'flag', false)).toBe(true)
        expect(readBooleanParam({ flag: 'false' }, 'flag', true)).toBe(false)
        expect(readBooleanParam({}, 'flag', true)).toBe(true)
        expect(readBooleanParam({}, 'flag', false)).toBe(false)
        expect(readBooleanParam({ flag: 'yes' }, 'flag', true)).toBe(true)
        expect(readBooleanParam({ flag: ['false', 'true'] }, 'flag', true)).toBe(false)
    })

    test('writeBooleanParam 은 기본값과 같으면 주소에서 뺀다', () => {
        expect(writeBooleanParam(true, true)).toBeUndefined()
        expect(writeBooleanParam(false, false)).toBeUndefined()
        expect(writeBooleanParam(false, true)).toBe('false')
        expect(writeBooleanParam(true, false)).toBe('true')
    })

    test('readListParam 은 콤마 문자열을 배열로 되돌린다', () => {
        expect(readListParam({ lamp: 'Hard,ExHard' }, 'lamp')).toEqual(['Hard', 'ExHard'])
        expect(readListParam({}, 'lamp')).toEqual([])
    })
})
