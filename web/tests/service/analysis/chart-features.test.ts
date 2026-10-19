import { describe, expect, test } from 'bun:test'
import { ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { computeChartFeatures, toPatternInput } from '@server/service/domain/analysis/chart-features'
import { LAYOUT_DEPENDENT_TAGS } from '@server/service/domain/analysis/pattern-tags'
import { makePatternDocument, type FixtureNote } from '../../support/chart-pattern-fixture'

const S16 = 100_000
const SECOND = 1_000_000

const TRILL: FixtureNote[] = [0, 1, 0, 1].map((lane, index) => [index * S16, lane, 0])
const LONG_NOTE: FixtureNote[] = [
    [2 * SECOND, 3, 1],
    [3 * SECOND, 3, 2],
]
const SCRATCH: FixtureNote = [4 * SECOND, 7, 0]
const MINE: FixtureNote = [5 * SECOND, 5, 3]

describe('곡별 특징(chart_pattern.features)', () => {
    test('원본 배치 기준임을 표시하고 판정 노트 수와 태그별 노트 수·비율을 담는다', () => {
        const features = computeChartFeatures(makePatternDocument([...TRILL, ...LONG_NOTE, SCRATCH, MINE]))
        expect(features).toMatchObject({ analysis_version: ANALYSIS_VERSION, layout: 'original', judged_notes: 6, ln_end_judged: false, mines: 1 })
        expect(features.layout_dependent_tags).toEqual(LAYOUT_DEPENDENT_TAGS)
        expect(features.layout_dependent_tags).toContain('trill')
        expect(features.layout_dependent_tags).not.toContain('chord_2')
        expect(features.tags.trill).toEqual({ notes: 4, ratio: 0.6667 })
        expect(features.tags.ln_head).toEqual({ notes: 1, ratio: 0.1667 })
        expect(features.tags.ln_end).toEqual({ notes: 0, ratio: 0 })
        expect(features.tags.scratch_solo).toEqual({ notes: 1, ratio: 0.1667 })
        expect(features.ln_ratio).toBe(0.1667)
        expect(features.scratch_ratio).toBe(0.1667)
        expect(features.length_ms).toBe(5000)
    })

    test('CN·HCN 으로 선언한 차트는 LN 끝을 판정 노트로 센다', () => {
        const features = computeChartFeatures(makePatternDocument([...TRILL, ...LONG_NOTE], { declared_ln: 'hcn' }))
        expect(features.ln_end_judged).toBe(true)
        expect(features.judged_notes).toBe(6)
        expect(features.tags.ln_end.notes).toBe(1)
        expect(toPatternInput(makePatternDocument(LONG_NOTE, { declared_ln: 'cn' })).notes.map((note) => note.kind)).toEqual([
            'ln_head',
            'charge_end',
        ])
    })

    test('밀도는 1초 창 판정 노트 수의 최고·평균·후반 평균이다', () => {
        const notes: FixtureNote[] = [
            ...Array.from({ length: 6 }, (_, index): FixtureNote => [index * S16, index, 0]),
            [SECOND, 0, 0],
            [2 * SECOND, 0, 0],
            [2 * SECOND + S16, 1, 0],
        ]
        expect(computeChartFeatures(makePatternDocument(notes)).density).toEqual({ peak: 6, average: 3, late_average: 2 })
    })

    test('소프란 횟수와 BPM 최소·최대 비를 센다', () => {
        const features = computeChartFeatures(
            makePatternDocument(TRILL, {
                tempo: [
                    { t_us: SECOND, bpm: 300, stop_us: 0, scroll: 1 },
                    { t_us: 2 * SECOND, bpm: 300, stop_us: 250_000, scroll: 1 },
                    { t_us: 3 * SECOND, bpm: 75, stop_us: 0, scroll: 1 },
                ],
            }),
        )
        expect(features.soflan).toEqual({ tempo_jumps: 2, long_stops: 1, bpm: { min: 75, max: 300, max_to_min: 4 } })
    })

    test('노트가 없는 차트도 0 으로 계산된다', () => {
        const features = computeChartFeatures(makePatternDocument([]))
        expect(features).toMatchObject({ judged_notes: 0, length_ms: 0, ln_ratio: 0, density: { peak: 0, average: 0, late_average: 0 } })
    })
})
