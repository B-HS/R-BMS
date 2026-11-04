import { describe, expect, test } from 'bun:test'
import { bestStreakOf, currentStreakOf, thisWeekOf } from '@server/service/domain/dashboard/play-streak'

const daysOf = (plays: number[]) => plays.map((count) => ({ plays: count }))

describe('currentStreakOf', () => {
    test('오늘까지 이어진 연속 일 수를 센다', () => {
        expect(currentStreakOf(daysOf([0, 2, 1, 3]))).toBe(3)
        expect(currentStreakOf(daysOf([4, 1, 1, 1]))).toBe(4)
    })

    test('오늘 판이 아직 없으면 어제부터 거꾸로 센다', () => {
        expect(currentStreakOf(daysOf([0, 2, 1, 0]))).toBe(2)
    })

    test('오늘도 어제도 판이 없으면 0 이다', () => {
        expect(currentStreakOf(daysOf([3, 3, 0, 0]))).toBe(0)
    })

    test('하루짜리 창과 빈 창을 다룬다', () => {
        expect(currentStreakOf(daysOf([1]))).toBe(1)
        expect(currentStreakOf(daysOf([0]))).toBe(0)
        expect(currentStreakOf([])).toBe(0)
    })

    test('창 전체가 이어져 있으면 창 길이를 돌려준다', () => {
        expect(currentStreakOf(daysOf([1, 1, 1, 1, 1]))).toBe(5)
    })
})

describe('bestStreakOf', () => {
    test('가장 긴 연속 구간을 돌려준다', () => {
        expect(bestStreakOf(daysOf([1, 1, 0, 1, 1, 1, 0, 1]))).toBe(3)
    })

    test('판이 없으면 0, 전부 있으면 창 길이다', () => {
        expect(bestStreakOf(daysOf([0, 0, 0]))).toBe(0)
        expect(bestStreakOf(daysOf([2, 2, 2]))).toBe(3)
        expect(bestStreakOf([])).toBe(0)
    })

    test('오늘의 현재 연속이 더 길면 그것이 최고다', () => {
        expect(bestStreakOf(daysOf([1, 0, 1, 1, 1]))).toBe(3)
    })
})

describe('thisWeekOf', () => {
    const run = (dates: string[]) => thisWeekOf(dates.map((date) => ({ date, plays: 1 }))).map((day) => day.date)

    test('오늘이 목요일이면 월요일부터 오늘까지 4일이다', () => {
        expect(run(['2026-10-31', '2026-11-01', '2026-11-02', '2026-11-03', '2026-11-04', '2026-11-05'])).toEqual([
            '2026-11-02',
            '2026-11-03',
            '2026-11-04',
            '2026-11-05',
        ])
    })

    test('오늘이 월요일이면 오늘 하루뿐이다', () => {
        expect(run(['2026-10-31', '2026-11-01', '2026-11-02'])).toEqual(['2026-11-02'])
    })

    test('오늘이 일요일이면 앞선 월요일부터 7일이다', () => {
        expect(run(['2026-11-01', '2026-11-02', '2026-11-03', '2026-11-04', '2026-11-05', '2026-11-06', '2026-11-07', '2026-11-08'])).toEqual([
            '2026-11-02',
            '2026-11-03',
            '2026-11-04',
            '2026-11-05',
            '2026-11-06',
            '2026-11-07',
            '2026-11-08',
        ])
    })

    test('창이 이번 주보다 짧으면 있는 날만 돌려준다', () => {
        expect(run(['2026-11-04', '2026-11-05'])).toEqual(['2026-11-04', '2026-11-05'])
        expect(run([])).toEqual([])
    })
})
