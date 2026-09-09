import { describe, expect, test } from 'bun:test'
import { formatCount, formatDateTime, formatHash, formatPercent } from '../../src/shared/lib/format'

describe('formatDateTime 은 실행 타임존과 무관하게 같은 문자열을 만든다', () => {
    test('UTC 2025-09-04 15:33:20 은 KST 로 하루 넘겨 표시된다', () => {
        expect(formatDateTime(1_757_000_000_000)).toBe('2025-09-05 00:33')
    })

    test('유닉스 epoch 0 은 KST 09:00 이다', () => {
        expect(formatDateTime(0)).toBe('1970-01-01 09:00')
    })

    test('숫자가 아닌 시각은 대시로 표시한다', () => {
        expect(formatDateTime(Number.NaN)).toBe('—')
    })
})

describe('format 유틸', () => {
    test('formatCount 는 천 단위 구분자를 넣는다', () => {
        expect(formatCount(18422)).toBe('18,422')
    })

    test('formatHash 는 8자로 잘라 말줄임표를 붙인다', () => {
        expect(formatHash('b1946ac92492d2347c6235b4d2611184')).toBe('b1946ac9…')
    })

    test('formatPercent 는 분모 0 을 0.00% 로 처리한다', () => {
        expect(formatPercent(3, 0)).toBe('0.00%')
        expect(formatPercent(1, 8)).toBe('12.50%')
    })
})
