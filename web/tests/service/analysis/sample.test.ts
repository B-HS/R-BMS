import { describe, expect, test } from 'bun:test'
import { MIN_TAG_NOTES, MIN_TAG_PLAYS, MIN_TIMING_NOTES, MIN_TIMING_PLAYS } from '@server/service/domain/analysis/analysis-constants'
import {
    EVIDENCE,
    TAG_LEANING,
    aggregateTagEvidence,
    timingEvidence,
    wilsonInterval,
    type TagPlaySample,
} from '@server/service/domain/analysis/sample'

const tally = (notes: number, exRatio: number) => ({ notes, ex_score: Math.round(notes * 2 * exRatio) })

const plays = (count: number, notesPerPlay: number, tagRatio: number, playRatio: number, scratchAuto = false): TagPlaySample[] =>
    Array.from({ length: count }, () => ({
        scratchAuto,
        overall: tally(notesPerPlay * 4, playRatio),
        byTag: { trill: tally(notesPerPlay, tagRatio), scratch_solo: tally(notesPerPlay, tagRatio) },
    }))

describe('윌슨 점수 구간', () => {
    test('100번 중 50번이면 약 0.404~0.596 이고 시행이 없으면 전체 범위다', () => {
        const interval = wilsonInterval(50, 100)
        expect(interval.low).toBeCloseTo(0.4038, 3)
        expect(interval.high).toBeCloseTo(0.5962, 3)
        expect(wilsonInterval(0, 0)).toEqual({ low: 0, high: 1 })
    })
})

describe('태그 약점·강점 판단(4.4·4.6)', () => {
    test('노트나 판 수가 모자라면 insufficient 이고 약점으로 부르지 않는다', () => {
        const fewPlays = aggregateTagEvidence(plays(MIN_TAG_PLAYS - 1, MIN_TAG_NOTES, 0.5, 0.8))
        expect(fewPlays.trill).toMatchObject({ grade: EVIDENCE.INSUFFICIENT, leaning: null, plays: MIN_TAG_PLAYS - 1 })
        const fewNotes = aggregateTagEvidence(plays(MIN_TAG_PLAYS, Math.floor((MIN_TAG_NOTES - 1) / MIN_TAG_PLAYS), 0.5, 0.8))
        expect(fewNotes.trill.grade).toBe(EVIDENCE.INSUFFICIENT)
    })

    test('표본이 넉넉하고 구간이 판 기준선을 벗어나면 strong 이고 잔차 부호로 약점·강점을 가린다', () => {
        const weak = aggregateTagEvidence(plays(MIN_TAG_PLAYS, 100, 0.6, 0.8)).trill
        expect(weak).toMatchObject({ grade: EVIDENCE.STRONG, leaning: TAG_LEANING.WEAKNESS, ex_ratio: 0.6, baseline_ex_ratio: 0.8, residual: -0.2 })
        const strong = aggregateTagEvidence(plays(MIN_TAG_PLAYS, 100, 0.95, 0.8)).trill
        expect(strong).toMatchObject({ grade: EVIDENCE.STRONG, leaning: TAG_LEANING.STRENGTH })
    })

    test('차이가 작아 구간이 기준선을 품으면 uncertain 이다', () => {
        expect(aggregateTagEvidence(plays(MIN_TAG_PLAYS, 100, 0.79, 0.8)).trill).toMatchObject({ grade: EVIDENCE.UNCERTAIN, leaning: null })
    })

    test('SCRATCH AUTO 판은 스크래치 태그 표본에서 빠지고 auto_plays 로 센다', () => {
        const evidence = aggregateTagEvidence([...plays(MIN_TAG_PLAYS, 100, 0.6, 0.8), ...plays(3, 100, 0.6, 0.8, true)])
        expect(evidence.scratch_solo).toMatchObject({ plays: MIN_TAG_PLAYS, auto_plays: 3 })
        expect(evidence.trill).toMatchObject({ plays: MIN_TAG_PLAYS + 3, auto_plays: 0 })
    })
})

describe('오차 습관 표본(4.6)', () => {
    test('판 10개와 PG·GR 2,000개가 모두 있어야 오프셋 숫자를 낸다', () => {
        expect(timingEvidence({ plays: MIN_TIMING_PLAYS, notes: MIN_TIMING_NOTES })).toBe(EVIDENCE.STRONG)
        expect(timingEvidence({ plays: MIN_TIMING_PLAYS - 1, notes: MIN_TIMING_NOTES })).toBe(EVIDENCE.INSUFFICIENT)
        expect(timingEvidence({ plays: MIN_TIMING_PLAYS, notes: MIN_TIMING_NOTES - 1 })).toBe(EVIDENCE.INSUFFICIENT)
    })
})
