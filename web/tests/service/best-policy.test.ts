import { describe, expect, test } from 'bun:test'
import { judgeBreakdownSchema } from '@server/dto/common'
import { deriveExScore, deriveMinbp, isBetterBest } from '@server/service/domain/score/best-policy'

const judge = (overrides: Record<string, number>) =>
    judgeBreakdownSchema.parse({
        pgreat: 0,
        great: 0,
        good: 0,
        bad: 0,
        poor: 0,
        miss: 0,
        fast: 0,
        slow: 0,
        combobreak: 0,
        ...overrides,
    })

describe('deriveExScore', () => {
    test('early/late 분리값에서 EX 를 유도한다', () => {
        expect(deriveExScore(judge({ epg: 700, lpg: 12, egr: 50, lgr: 14 }))).toBe(1488)
    })

    test('분리값이 없으면 합계 필드로 유도한다', () => {
        expect(deriveExScore(judge({ pgreat: 712, great: 64 }))).toBe(1488)
    })
})

describe('deriveMinbp', () => {
    test('BAD+POOR+MISS 를 early/late 합으로 계산한다', () => {
        expect(deriveMinbp(judge({ ebd: 1, lbd: 7, epr: 2, lpr: 3, ems: 0, lms: 2 }))).toBe(15)
    })

    test('분리값이 없으면 합계 필드로 계산한다', () => {
        expect(deriveMinbp(judge({ bad: 8, poor: 5, miss: 2 }))).toBe(15)
    })
})

describe('isBetterBest (램프 > EX > BP)', () => {
    test('기존 기록이 없으면 항상 갱신한다', () => {
        expect(isBetterBest({ clear: 0, exScore: 0, minbp: 999 }, null)).toBe(true)
    })

    test('램프가 높으면 EX 가 낮아도 갱신한다', () => {
        expect(isBetterBest({ clear: 6, exScore: 1000, minbp: 50 }, { clear: 5, exScore: 2000, minbp: 0 })).toBe(true)
    })

    test('램프가 낮으면 EX 가 높아도 갱신하지 않는다', () => {
        expect(isBetterBest({ clear: 4, exScore: 2500, minbp: 0 }, { clear: 5, exScore: 1000, minbp: 90 })).toBe(false)
    })

    test('램프가 같으면 EX 로 비교한다', () => {
        expect(isBetterBest({ clear: 5, exScore: 1501, minbp: 30 }, { clear: 5, exScore: 1500, minbp: 1 })).toBe(true)
        expect(isBetterBest({ clear: 5, exScore: 1499, minbp: 0 }, { clear: 5, exScore: 1500, minbp: 99 })).toBe(false)
    })

    test('램프와 EX 가 같으면 BP 가 낮은 쪽이 이긴다', () => {
        expect(isBetterBest({ clear: 5, exScore: 1500, minbp: 3 }, { clear: 5, exScore: 1500, minbp: 4 })).toBe(true)
        expect(isBetterBest({ clear: 5, exScore: 1500, minbp: 4 }, { clear: 5, exScore: 1500, minbp: 4 })).toBe(false)
    })
})
