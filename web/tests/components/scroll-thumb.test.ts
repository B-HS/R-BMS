import { describe, expect, test } from 'bun:test'
import { calculateScrollThumb, IDLE_SCROLL_THUMB } from '@shared/lib/scroll-thumb'

describe('calculateScrollThumb', () => {
    test('스크롤할 내용이 없으면 표시하지 않는다', () => {
        expect(calculateScrollThumb({ scrollTop: 0, scrollHeight: 500, clientHeight: 500 })).toEqual(IDLE_SCROLL_THUMB)
    })

    test('높이가 0이거나 내용이 없어도 NaN 없이 표시하지 않는다', () => {
        expect(calculateScrollThumb({ scrollTop: 0, scrollHeight: 0, clientHeight: 0 })).toEqual(IDLE_SCROLL_THUMB)
        expect(calculateScrollThumb({ scrollTop: 0, scrollHeight: 400, clientHeight: 0 })).toEqual(IDLE_SCROLL_THUMB)
        expect(calculateScrollThumb({ scrollTop: Number.NaN, scrollHeight: 400, clientHeight: 200 }).topPercent).not.toBeNaN()
    })

    test('보이는 비율만큼 막대 높이를 정한다', () => {
        const thumb = calculateScrollThumb({ scrollTop: 0, scrollHeight: 1000, clientHeight: 500 })
        expect(thumb).toEqual({ heightPercent: 50, topPercent: 0, isScrollable: true })
    })

    test('끝까지 스크롤하면 막대가 아래 끝에 닿는다', () => {
        const thumb = calculateScrollThumb({ scrollTop: 500, scrollHeight: 1000, clientHeight: 500 })
        expect(thumb.topPercent + thumb.heightPercent).toBe(100)
    })

    test('내용이 매우 길어도 막대 최소 높이를 지킨다', () => {
        const thumb = calculateScrollThumb({ scrollTop: 0, scrollHeight: 100000, clientHeight: 500 })
        expect(thumb.heightPercent).toBe(10)
    })

    test('범위를 벗어난 스크롤 위치는 막대 범위 안으로 맞춘다', () => {
        const overscrolled = calculateScrollThumb({ scrollTop: -40, scrollHeight: 1000, clientHeight: 500 })
        const pastEnd = calculateScrollThumb({ scrollTop: 900, scrollHeight: 1000, clientHeight: 500 })
        expect(overscrolled.topPercent).toBe(0)
        expect(pastEnd.topPercent + pastEnd.heightPercent).toBe(100)
    })
})
