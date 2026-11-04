import { describe, expect, test } from 'bun:test'
import {
    trainingBpmChangesOf,
    trainingBpmDefinitionsOf,
    trainingTimelineOf,
    trainingTokenOf,
    type WrittenLine,
} from '@server/service/domain/training/chart-timing'

const line = (measure: number, channel: string, tokens: string[]): WrittenLine => ({ measure, channel, resolution: tokens.length, tokens })

const every = (count: number, token = '01') => Array.from({ length: count }, () => token)

describe('훈련곡 시각 계산 (trainingTimelineOf)', () => {
    test('BPM 160 은 마디가 1.5초이고 마디 39 의 3/4 는 02 부록 A 의 클라이언트 측정값 59,625,000µs 다', () => {
        const lines = Array.from({ length: 40 }, (_, measure) => line(measure, '11', every(16)))
        const { timeAt, tempo } = trainingTimelineOf(lines, 160, [])
        expect(timeAt(1)).toBe(1_500_000)
        expect(timeAt(39 + 3 / 4)).toBe(59_625_000)
        expect(timeAt(39 + 15 / 16)).toBe(59_906_250)
        expect(tempo).toEqual([])
    })

    test('마이크로초가 정수가 아닌 BPM 175·12분은 점마다 절사를 누적한다', () => {
        const lines = Array.from({ length: 40 }, (_, measure) => line(measure, '11', every(12)))
        const { timeAt } = trainingTimelineOf(lines, 175, [])
        const positions = Array.from({ length: 40 * 12 }, (_, index) => Math.floor(index / 12) + (index % 12) / 12)
        const expected = positions.reduce(
            (time, position, index) => (index === 0 ? 0 : time + Math.trunc((240_000_000 * (position - positions[index - 1])) / 175)),
            0,
        )
        expect(timeAt(39 + 11 / 12)).toBe(expected)
        expect(timeAt(39 + 11 / 12)).toBe(54_742_515)
        expect(timeAt(39 + 11 / 12)).toBeLessThan(Math.round(((39 + 11 / 12) * 240_000_000) / 175))
    })

    test('채널 08 의 BPM 은 그 점부터 적용되고 tempo 점으로 나온다', () => {
        const definitions = trainingBpmDefinitionsOf([
            { measure: 2, bpm: 240 },
            { measure: 3, bpm: 120 },
        ])
        expect(definitions).toEqual([
            { id: '01', bpm: 120 },
            { id: '02', bpm: 240 },
        ])
        const lines = [line(0, '01', every(4, '02')), line(2, '08', ['02']), line(3, '08', ['01']), line(3, '11', ['00', '01'])]
        const { timeAt, tempo } = trainingTimelineOf(lines, 120, definitions)
        expect(timeAt(2)).toBe(4_000_000)
        expect(timeAt(3)).toBe(5_000_000)
        expect(timeAt(3.5)).toBe(6_000_000)
        expect(timeAt(4)).toBe(7_000_000)
        expect(tempo).toEqual([
            { t_us: 4_000_000, bpm: 240 },
            { t_us: 5_000_000, bpm: 120 },
        ])
    })

    test('BPM 변화는 구간 시작마다 이전 값과 다를 때만 생기고 토큰은 36진 두 자리다', () => {
        expect(
            trainingBpmChangesOf({
                base_bpm: 160,
                sections: [
                    { measure_from: 1, bpm: 160 },
                    { measure_from: 5, bpm: 240 },
                    { measure_from: 9, bpm: 240 },
                    { measure_from: 12, bpm: 160 },
                ],
            }),
        ).toEqual([
            { measure: 5, bpm: 240 },
            { measure: 12, bpm: 160 },
        ])
        expect([1, 9, 10, 35].map(trainingTokenOf)).toEqual(['01', '09', '0A', '0Z'])
    })
})
