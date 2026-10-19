import { describe, expect, test } from 'bun:test'
import { SEGMENT_COUNT } from '@server/service/domain/analysis/analysis-constants'
import {
    JUDGE_STATE,
    computePlayMetrics,
    describeGreenNumber,
    findGaugeDrops,
    readTraceGreenNumber,
    summarizeGaugeCurves,
    summarizeTiming,
    type JudgedPlayNote,
} from '@server/service/domain/analysis/play-metrics'

const BPM = 150
const S16 = 100_000
const SECOND = 1_000_000
const LENGTH_US = 32 * SECOND

const played = (timeUs: number, lane: number, state: number, errUs = 0, kind: JudgedPlayNote['kind'] = 'normal'): JudgedPlayNote => ({
    timeUs,
    lane,
    kind,
    state,
    errUs,
})

const TRILL = [0, 1, 0, 1].map((lane, index) => played(index * S16, lane, JUDGE_STATE.PGREAT, 1500))

const metricsOf = (notes: JudgedPlayNote[], emptyPoorTimesUs: number[] = []) =>
    computePlayMetrics({ notes, tempo: [], initBpm: BPM, emptyPoorTimesUs, lengthUs: LENGTH_US })

describe('판 하나의 지표(4.3)', () => {
    test('판정 비율과 EX 비율은 판정을 받은 노트만 세고, 판정 안 하는 LN 끝과 도달 못한 노트는 뺀다', () => {
        const metrics = metricsOf([
            ...TRILL,
            played(SECOND, 3, JUDGE_STATE.GREAT, -2000, 'ln_head'),
            played(2 * SECOND, 3, JUDGE_STATE.NONE, 0, 'ln_end'),
            played(3 * SECOND, 4, JUDGE_STATE.NONE),
        ])
        expect(metrics.layout).toBe('played')
        expect(metrics.overall).toMatchObject({ notes: 5, pgreat: 4, great: 1, ex_score: 9, ex_ratio: 0.9 })
        expect(metrics.overall.fast).toEqual({ pgreat: 4, great: 0, good: 0, bad: 0 })
        expect(metrics.overall.slow).toEqual({ pgreat: 0, great: 1, good: 0, bad: 0 })
        expect(metrics.by_tag.trill).toMatchObject({ notes: 4, ex_ratio: 1 })
        expect(metrics.by_tag.ln_head).toMatchObject({ notes: 1, ex_ratio: 0.5 })
        expect(metrics.by_lane['1']).toMatchObject({ notes: 2 })
        expect(metrics.by_lane['4']).toMatchObject({ notes: 1 })
    })

    test('구간별 BP 는 BAD·POOR·空POOR 를 세고 콤보 끊김은 이어지던 콤보가 끊긴 횟수다', () => {
        const metrics = metricsOf(
            [
                played(0, 0, JUDGE_STATE.PGREAT),
                played(S16, 1, JUDGE_STATE.BAD, 30_000),
                played(2 * S16, 2, JUDGE_STATE.POOR),
                played(5 * SECOND, 0, JUDGE_STATE.GOOD, -40_000),
                played(31 * SECOND, 0, JUDGE_STATE.POOR),
            ],
            [5 * SECOND + 1],
        )
        expect(metrics.segments).toHaveLength(SEGMENT_COUNT)
        expect(metrics.segments[0]).toEqual({ id: 's00', bp: 2, combo_breaks: 1 })
        expect(metrics.segments[5]).toEqual({ id: 's05', bp: 1, combo_breaks: 0 })
        expect(metrics.segments[31]).toEqual({ id: 's31', bp: 1, combo_breaks: 1 })
        expect(metrics.empty_poor).toBe(1)
        expect(metrics.overall.fast.bad).toBe(1)
        expect(metrics.overall.slow.good).toBe(1)
    })

    test('밀도 칸은 1초 창의 판정 노트 수를 5개 단위로 묶는다', () => {
        const metrics = metricsOf([...TRILL, played(10 * SECOND, 0, JUDGE_STATE.GREAT)])
        expect(Object.keys(metrics.by_density).toSorted()).toEqual(['0-4'])
        expect(metrics.by_density['0-4']).toMatchObject({ notes: 5 })
    })

    test('피로는 앞 1/3 과 뒤 1/3 의 EX 비율 차이다', () => {
        const metrics = metricsOf([played(SECOND, 0, JUDGE_STATE.PGREAT), played(30 * SECOND, 0, JUDGE_STATE.GREAT)])
        expect(metrics.fatigue).toEqual({ first_third_ex_ratio: 1, last_third_ex_ratio: 0.5, delta: -0.5 })
    })

    test('도달하지 못해 판정이 없는 구간의 피로는 0 이 아니라 null 이라, 앞 구간과 비교하지 않는다', () => {
        const unreachedLast = metricsOf([played(SECOND, 0, JUDGE_STATE.PGREAT), played(30 * SECOND, 0, JUDGE_STATE.NONE)])
        expect(unreachedLast.fatigue).toEqual({ first_third_ex_ratio: 1, last_third_ex_ratio: null, delta: null })
        const unreachedFirst = metricsOf([played(SECOND, 0, JUDGE_STATE.NONE), played(30 * SECOND, 0, JUDGE_STATE.GREAT)])
        expect(unreachedFirst.fatigue).toEqual({ first_third_ex_ratio: null, last_third_ex_ratio: 0.5, delta: null })
        expect(metricsOf([]).fatigue).toEqual({ first_third_ex_ratio: null, last_third_ex_ratio: null, delta: null })
    })

    test('노트가 있는데 전부 틀려 EX 가 0 인 구간은 데이터 없음이 아니라 비율 0 이다', () => {
        const metrics = metricsOf([played(SECOND, 0, JUDGE_STATE.PGREAT), played(30 * SECOND, 0, JUDGE_STATE.POOR)])
        expect(metrics.fatigue).toEqual({ first_third_ex_ratio: 1, last_third_ex_ratio: 0, delta: -1 })
    })
})

describe('오차 분포', () => {
    test('평균·중앙값·표준편차·사분위 범위와 1ms 히스토그램(0 쪽으로 버림, ±150ms 안만)', () => {
        const timing = summarizeTiming([-2500, -500, 500, 1500, 200_000])
        expect(timing).toMatchObject({ notes: 5, mean_us: 39_800, median_us: 500, iqr_us: 2000 })
        expect(timing.histogram_ms).toEqual([
            { ms: -2, notes: 1 },
            { ms: 0, notes: 2 },
            { ms: 1, notes: 1 },
        ])
        expect(summarizeTiming([])).toMatchObject({ notes: 0, mean_us: 0, stdev_us: 0, histogram_ms: [] })
    })
})

describe('게이지 곡선', () => {
    const curve = { intervalMs: 500, min: 0, max: 100, border: 80 }

    test('4초 창 안에서 범위의 15% 이상 떨어진 곳을 한 번의 꺾임으로 묶는다', () => {
        const values = [100, 100, 100, 95, 88, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 60, 60]
        expect(findGaugeDrops({ ...curve, values })).toEqual([
            { from_ms: 1000, to_ms: 2500, drop: 20 },
            { from_ms: 10_000, to_ms: 10_500, drop: 20 },
        ])
        expect(findGaugeDrops({ ...curve, values: [100, 95, 90, 95, 100] })).toEqual([])
    })

    test('게이지마다 끝값·클리어 여부·바닥에 닿았는지를 읽기만 한다', () => {
        expect(
            summarizeGaugeCurves([
                { ...curve, values: [20, 50, 82] },
                { ...curve, values: [100, 0, 0] },
            ]),
        ).toEqual([
            { end: 82, cleared: true, reached_min: false },
            { end: 0, cleared: false, reached_min: true },
        ])
    })
})

describe('게이지 곡선 — 클리어 판정', () => {
    const groove = { intervalMs: 500, min: 2, max: 100, border: 80 }
    const survival = { intervalMs: 500, min: 0, max: 100, border: 0 }

    test('생존 게이지(최소 0, border 0)는 끝값이 0 보다 크면 클리어, 0 이면 아니다', () => {
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 40, 3.2] }])).toEqual([{ end: 3.2, cleared: true, reached_min: false }])
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 40, 0, 0] }])).toEqual([{ end: 0, cleared: false, reached_min: true }])
    })

    test('생존 게이지가 실패 시각 정보를 갖고 있으면 끝값이 0 보다 커도 실패다', () => {
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 60, 30], failedAtMs: 1000 }])).toEqual([
            { end: 30, cleared: false, reached_min: false },
        ])
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 60, 30], failedAtMs: null }])[0]).toMatchObject({ cleared: true })
    })

    test('그 밖의 게이지는 끝값이 border 이상일 때만 클리어이고, 실패 시각이 있으면 그것이 먼저다', () => {
        expect(
            summarizeGaugeCurves([
                { ...groove, values: [20, 80] },
                { ...groove, values: [20, 79.99] },
            ]).map((summary) => summary.cleared),
        ).toEqual([true, false])
        expect(summarizeGaugeCurves([{ ...groove, values: [20, 90], failedAtMs: 500 }])[0]).toMatchObject({ cleared: false })
    })
})

describe('녹색 숫자', () => {
    test('점수 행만 있으면 레인 통과 ms 에서 GREEN 값(× 0.6)을 어림해 추정으로 표시한다', () => {
        expect(describeGreenNumber(500)).toEqual({ green: 300, lane_ms: 500, is_estimated: true })
        expect(describeGreenNumber(1000.4)).toEqual({ green: 600, lane_ms: 1000, is_estimated: true })
    })

    test('trace 의 green 값은 클라이언트가 스킨과 같은 식으로 낸 값이라 다시 계산하지 않고 그대로 쓴다', () => {
        expect(readTraceGreenNumber({ green: 481, greenMs: 801 })).toEqual({ green: 481, lane_ms: 801, is_estimated: false })
        expect(readTraceGreenNumber({ green: 480, greenMs: 799 })).toEqual({ green: 480, lane_ms: 799, is_estimated: false })
    })
})
