import { describe, expect, test } from 'bun:test'
import { getDjRank, getDjRankClassName, getRateStep } from '../../src/shared/lib/dj-rank'

const STEPPED_MAX_EX = 27 * 40

describe('getDjRank 27단계 랭크', () => {
    test('만점은 MAX, 0점은 F- 다', () => {
        expect(getDjRank(STEPPED_MAX_EX, STEPPED_MAX_EX)).toBe('MAX')
        expect(getDjRank(0, STEPPED_MAX_EX)).toBe('F-')
    })

    test('경계에 정확히 선 점수는 그 단계에, 1점 모자라면 아래 단계에 든다', () => {
        for (let step = 1; step <= 27; step += 1) {
            const at = (STEPPED_MAX_EX * step) / 27
            expect(getRateStep(at, STEPPED_MAX_EX)).toBe(step)
            expect(getRateStep(at - 1, STEPPED_MAX_EX)).toBe(step - 1)
        }
    })

    test('앱의 이름 표와 같은 이름을 낸다', () => {
        const at = (step: number) => getDjRank((STEPPED_MAX_EX * step) / 27, STEPPED_MAX_EX)
        expect([0, 2, 4, 6, 7, 8].map(at)).toEqual(['F-', 'F', 'F+', 'E-', 'E', 'E+'])
        expect([18, 19, 20, 21, 22, 23, 24, 25, 26].map(at)).toEqual(['A-', 'A', 'A+', 'AA-', 'AA', 'AA+', 'AAA-', 'AAA', 'MAX-'])
    })

    test('노트 수가 0 이거나 점수가 넘쳐도 범위를 벗어나지 않는다', () => {
        expect(getDjRank(10, 0)).toBe('F-')
        expect(getDjRank(STEPPED_MAX_EX * 2, STEPPED_MAX_EX)).toBe('MAX')
    })

    test('랭크는 색이 아니라 굵기와 명도로 구분한다', () => {
        expect(getDjRankClassName(STEPPED_MAX_EX, STEPPED_MAX_EX)).toBe('font-semibold text-foreground')
        expect(getDjRankClassName((STEPPED_MAX_EX * 20) / 27, STEPPED_MAX_EX)).toBe('font-medium text-foreground')
        expect(getDjRankClassName(0, STEPPED_MAX_EX)).toBe('text-muted-foreground')
    })
})
