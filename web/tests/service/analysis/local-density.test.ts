import { describe, expect, test } from 'bun:test'
import { localDensitiesOf, npsBinKeyOf, npsBinOf, peakNpsOf, peakRowsPerSecondOf } from '@server/service/domain/analysis/local-density'
import type { PatternNote } from '@server/service/domain/analysis/pattern-tags'

const SECOND = 1_000_000
const TENTH = 100_000
const QUARTER = 250_000

const hit = (timeUs: number, lane = 0, kind: PatternNote['kind'] = 'normal'): PatternNote => ({ timeUs, lane, kind })

/** One hit every `stepUs` from 0 up to `untilUs` inclusive, lanes cycling 0 to 6. */
const streamOf = (stepUs: number, untilUs: number) => Array.from({ length: untilUs / stepUs + 1 }, (_, index) => hit(index * stepUs, index % 7))

describe('국소 NPS(2.2.1)', () => {
    test('곡 가운데 노트는 1초·2초·8초 중심 창의 판정 노트 수를 초로 나눈 값이고, 10 NPS 가 이어지면 sustained 다', () => {
        const notes = streamOf(TENTH, 20 * SECOND)
        const middle = localDensitiesOf(notes)[100]
        expect(middle).toEqual({ nps1: 10, nps2: 10, nps8: 10, context: 'sustained' })
    })

    test('곡 처음의 창은 0 에서 잘라 자른 길이로 나누므로 밀도를 낮게 읽지 않는다', () => {
        const first = localDensitiesOf(streamOf(TENTH, 20 * SECOND))[0]
        expect(first).toMatchObject({ nps1: 10, nps2: 10, nps8: 10 })
    })

    test('1초 창이 8초 창의 1.5배 이상이면 burst, 6 NPS 미만이 고르게 이어지면 mixed 다', () => {
        const sparse = Array.from({ length: 20 }, (_, index) => hit((2 * index + 1) * SECOND, 5))
        const burst = [0, 1, 2, 3].map((step) => hit(20 * SECOND + step * TENTH, step))
        const notes = [...sparse, ...burst].toSorted((a, b) => a.timeUs - b.timeUs)
        const densities = localDensitiesOf(notes)
        expect(densities[notes.indexOf(burst[1])]).toEqual({ nps1: 4, nps2: 2.5, nps8: 1, context: 'burst' })
        const even = localDensitiesOf(streamOf(QUARTER, 20 * SECOND))[40]
        expect(even).toEqual({ nps1: 4, nps2: 4, nps8: 4, context: 'mixed' })
    })

    test('지뢰와 판정 안 하는 LN 끝은 세지 않지만 그 노트에도 밀도가 붙는다', () => {
        const stream = streamOf(TENTH, 20 * SECOND)
        const withOthers = [...stream, hit(10 * SECOND, 6, 'mine'), hit(10 * SECOND + 1, 3, 'ln_end')].toSorted((a, b) => a.timeUs - b.timeUs)
        const densities = localDensitiesOf(withOthers)
        expect(densities).toHaveLength(withOthers.length)
        expect(densities[withOthers.findIndex((note) => note.kind === 'mine')]).toMatchObject({ nps2: 10 })
        expect(densities[withOthers.indexOf(stream[100])]).toEqual(localDensitiesOf(stream)[100])
    })

    test('노트 순서가 시각 순이 아니어도 노트마다 같은 밀도를 돌려준다', () => {
        const notes = streamOf(TENTH, 20 * SECOND)
        expect(localDensitiesOf(notes.toReversed()).toReversed()).toEqual(localDensitiesOf(notes))
    })
})

describe('NPS 칸', () => {
    test('1 NPS 단위 칸이고 40 이상은 한 칸이다', () => {
        expect([0.5, 1, 9.99, 39.9, 40, 75].map(npsBinOf)).toEqual([0, 1, 9, 39, 40, 40])
        expect([0, 39, 40].map(npsBinKeyOf)).toEqual(['0', '39', '40+'])
    })
})

describe('최고 NPS·행 밀도', () => {
    test('노트에서 시작하는 창마다 판정 노트 수의 최댓값을 창 길이(초)로 나누고, 행은 같은 시각을 하나로 센다', () => {
        const notes = [
            hit(20 * SECOND, 0),
            hit(20 * SECOND, 3),
            ...[1, 2, 3].map((step) => hit(20 * SECOND + step * TENTH, step)),
            hit(30 * SECOND, 1, 'mine'),
        ]
        expect(peakNpsOf(notes)).toEqual({ '500': 10, '1000': 5, '2000': 2.5, '4000': 1.25, '8000': 0.63 })
        expect(peakRowsPerSecondOf(notes)).toBe(2)
        expect(peakNpsOf([])).toEqual({ '500': 0, '1000': 0, '2000': 0, '4000': 0, '8000': 0 })
    })
})
