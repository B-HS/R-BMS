import { describe, expect, test } from 'bun:test'
import {
    MONOTONE,
    benjaminiHochberg,
    clusterRobustTest,
    firstCrossingOf,
    isotonicRegression,
    logGamma,
    regularizedIncompleteBeta,
    studentTCdf,
    twoSidedTPValue,
} from '@server/service/domain/analysis/profile-stats'

const CLOSE_DIGITS = 6

describe('t 분포(정규화 불완전 베타)', () => {
    test('자유도 1·4·30 의 양측 임계값은 알려진 p 값과 같다', () => {
        expect(twoSidedTPValue(12.7062047, 1)).toBeCloseTo(0.05, CLOSE_DIGITS)
        expect(twoSidedTPValue(1, 1)).toBeCloseTo(0.5, CLOSE_DIGITS)
        expect(twoSidedTPValue(2.776445, 4)).toBeCloseTo(0.05, CLOSE_DIGITS)
        expect(twoSidedTPValue(-2.776445, 4)).toBeCloseTo(0.05, CLOSE_DIGITS)
        expect(twoSidedTPValue(2.042272, 30)).toBeCloseTo(0.05, CLOSE_DIGITS)
        expect(twoSidedTPValue(2.749996, 30)).toBeCloseTo(0.01, CLOSE_DIGITS)
        expect(twoSidedTPValue(0, 10)).toBe(1)
    })

    test('누적분포는 대칭이고 단측 임계값과 맞는다', () => {
        expect(studentTCdf(1.812461, 10)).toBeCloseTo(0.95, CLOSE_DIGITS)
        expect(studentTCdf(-1, 1)).toBeCloseTo(0.25, CLOSE_DIGITS)
        expect(studentTCdf(0, 7)).toBeCloseTo(0.5, CLOSE_DIGITS)
    })

    test('불완전 베타와 감마의 닫힌 꼴 값', () => {
        expect(regularizedIncompleteBeta(0.2, 1, 1)).toBeCloseTo(0.2, CLOSE_DIGITS)
        expect(regularizedIncompleteBeta(0.3, 2, 1)).toBeCloseTo(0.09, CLOSE_DIGITS)
        expect(regularizedIncompleteBeta(0.5, 3, 3)).toBeCloseTo(0.5, CLOSE_DIGITS)
        expect(regularizedIncompleteBeta(0, 2, 3)).toBe(0)
        expect(regularizedIncompleteBeta(1, 2, 3)).toBe(1)
        expect(logGamma(5)).toBeCloseTo(Math.log(24), CLOSE_DIGITS)
        expect(logGamma(0.5)).toBeCloseTo(Math.log(Math.sqrt(Math.PI)), CLOSE_DIGITS)
    })
})

describe('군집 강건 t 검정', () => {
    test('가중 평균과 sqrt(K/(K−1)) 보정 표준오차, 자유도 K−1 의 p 값', () => {
        const result = clusterRobustTest([
            { value: -0.1, weight: 1 },
            { value: -0.2, weight: 1 },
            { value: -0.3, weight: 2 },
        ])
        const estimate = (-0.1 - 0.2 - 0.6) / 4
        const spread = (-0.1 - estimate) ** 2 + (-0.2 - estimate) ** 2 + 4 * (-0.3 - estimate) ** 2
        const se = (Math.sqrt(3 / 2) * Math.sqrt(spread)) / 4
        expect(result.estimate).toBeCloseTo(estimate, CLOSE_DIGITS)
        expect(result.se).toBeCloseTo(se, CLOSE_DIGITS)
        expect(result.df).toBe(2)
        expect(result.pValue).toBeCloseTo(twoSidedTPValue(estimate / se, 2), CLOSE_DIGITS)
    })

    test('군집이 하나이거나 모든 군집이 같은 값이면 정보가 없어 p 값이 1 이다', () => {
        expect(clusterRobustTest([{ value: -0.5, weight: 10 }]).pValue).toBe(1)
        expect(clusterRobustTest(Array.from({ length: 6 }, () => ({ value: -0.2, weight: 3 }))).pValue).toBe(1)
        expect(clusterRobustTest([])).toMatchObject({ estimate: null, pValue: 1 })
    })
})

describe('Benjamini–Hochberg', () => {
    test('정렬한 p 가 i/m·q 를 넘지 않는 가장 큰 순위까지 모두 통과한다', () => {
        const pValues = [0.01, 0.04, 0.03, 0.005, 0.2]
        const result = benjaminiHochberg(pValues, 0.05)
        expect(result.map((entry) => entry.passed)).toEqual([true, true, true, true, false])
        expect(result.map((entry) => entry.threshold)).toEqual([0.02, 0.04, 0.03, 0.01, 0.05].map((value) => expect.closeTo(value, CLOSE_DIGITS)))
    })

    test('중간 순위가 넘어도 더 큰 순위가 기준을 지키면 그 앞이 모두 통과한다', () => {
        expect(benjaminiHochberg([0.001, 0.03, 0.035, 0.9], 0.1).map((entry) => entry.passed)).toEqual([true, true, true, false])
        expect(benjaminiHochberg([0.5, 0.6], 0.1).map((entry) => entry.passed)).toEqual([false, false])
        expect(benjaminiHochberg([], 0.1)).toEqual([])
    })
})

describe('가중 등위회귀(PAVA)', () => {
    test('증가 방향은 어긋난 이웃을 가중 평균으로 합친다', () => {
        expect(isotonicRegression([1, 3, 2, 4], [1, 1, 1, 1], MONOTONE.INCREASING)).toEqual([1, 2.5, 2.5, 4])
        expect(isotonicRegression([1, 3, 2, 4], [1, 1, 3, 1], MONOTONE.INCREASING)).toEqual([1, 2.25, 2.25, 4])
    })

    test('감소 방향과 이미 단조인 입력', () => {
        expect(
            isotonicRegression([0.9, 0.95, 0.8, 0.85, 0.6], [1, 1, 1, 1, 2], MONOTONE.DECREASING).map((value) => Number(value.toFixed(6))),
        ).toEqual([0.925, 0.925, 0.825, 0.825, 0.6])
        expect(isotonicRegression([5, 4, 3], [2, 2, 2], MONOTONE.DECREASING)).toEqual([5, 4, 3])
        expect(isotonicRegression([], [], MONOTONE.INCREASING)).toEqual([])
    })
})

describe('맞춤 곡선이 기준선을 처음 지나는 점', () => {
    const FALLING = [
        { x: 5, y: 0.9 },
        { x: 9, y: 0.8 },
        { x: 13, y: 0.6 },
    ]

    test('기준선이 이웃한 두 점 사이를 지나면 두 점을 잇는 직선 위의 위치를 준다', () => {
        expect(firstCrossingOf(FALLING, 0.7, MONOTONE.DECREASING)?.index).toBe(2)
        expect(firstCrossingOf(FALLING, 0.7, MONOTONE.DECREASING)?.x).toBeCloseTo(11, CLOSE_DIGITS)
        expect(firstCrossingOf(FALLING, 0.85, MONOTONE.DECREASING)?.x).toBeCloseTo(7, CLOSE_DIGITS)
        expect(firstCrossingOf(FALLING, 0.65, MONOTONE.DECREASING)?.x).toBeCloseTo(12, CLOSE_DIGITS)
    })

    test('점이 기준선에 정확히 닿으면 그 점의 위치이고, 한 점도 닿지 않으면 null 이다', () => {
        expect(firstCrossingOf(FALLING, 0.8, MONOTONE.DECREASING)).toEqual({ index: 1, x: 9 })
        expect(firstCrossingOf(FALLING, 0.5, MONOTONE.DECREASING)).toBeNull()
        expect(firstCrossingOf([], 0.5, MONOTONE.DECREASING)).toBeNull()
    })

    test('첫 점이 이미 기준선을 넘었으면 앞 이웃이 없어 첫 점의 위치이다', () => {
        expect(firstCrossingOf(FALLING, 0.95, MONOTONE.DECREASING)).toEqual({ index: 0, x: 5 })
    })

    test('증가 곡선은 기준선 이상이 된 첫 점과 그 앞 점 사이를 잇는다', () => {
        const rising = [
            { x: 5, y: 0 },
            { x: 9, y: 0.04 },
            { x: 13, y: 0.1 },
        ]
        expect(firstCrossingOf(rising, 0.08, MONOTONE.INCREASING)?.x).toBeCloseTo(9 + (0.04 / 0.06) * 4, CLOSE_DIGITS)
        expect(firstCrossingOf(rising, 0.04, MONOTONE.INCREASING)).toEqual({ index: 1, x: 9 })
        expect(firstCrossingOf(rising, 0, MONOTONE.INCREASING)).toEqual({ index: 0, x: 5 })
        expect(firstCrossingOf(rising, 0.11, MONOTONE.INCREASING)).toBeNull()
    })
})
