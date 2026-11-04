import { describe, expect, test } from 'bun:test'
import { SEGMENT_COUNT } from '@server/service/domain/analysis/analysis-constants'
import {
    JUDGE_STATE,
    computePlayMetrics,
    describeGreenNumber,
    findGaugeDrops,
    readTraceGreenNumber,
    GAUGE_OUTCOME,
    RUN_END,
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
    computePlayMetrics({ notes, tempo: [], initBpm: BPM, emptyPoorTimesUs, lengthUs: LENGTH_US, gaugeDropsMs: [] })

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
        expect(metrics.segments[0]).toMatchObject({ id: 's00', bp: 2, combo_breaks: 1 })
        expect(metrics.segments[5]).toMatchObject({ id: 's05', bp: 1, combo_breaks: 0 })
        expect(metrics.segments[31]).toMatchObject({ id: 's31', bp: 1, combo_breaks: 1 })
        expect(metrics.empty_poor).toBe(1)
        expect(metrics.overall.fast.bad).toBe(1)
        expect(metrics.overall.slow.good).toBe(1)
    })

    test('구간마다 시각 범위, 판정 객체 수(도달 못한 것 포함), 판정 받은 노트·EX, FAST/SLOW(GREAT~BAD), 많은 태그 셋을 남긴다', () => {
        const metrics = metricsOf([
            ...TRILL,
            played(S16 * 4, 2, JUDGE_STATE.GOOD, -3000),
            played(S16 * 5, 3, JUDGE_STATE.NONE),
            played(10 * SECOND, 0, JUDGE_STATE.GREAT, -700),
        ])
        expect(metrics.segments[0]).toMatchObject({
            from_ms: 0,
            to_ms: 1000,
            chart_notes: 6,
            notes: 5,
            ex_score: 8,
            fast: 0,
            slow: 1,
        })
        expect(metrics.segments[0].top_tags).toContainEqual({ tag: 'trill', notes: 4 })
        expect(metrics.segments[0].top_tags.length).toBeLessThanOrEqual(3)
        expect(metrics.segments[10]).toMatchObject({ from_ms: 10_000, chart_notes: 1, notes: 1, ex_score: 1, slow: 1 })
        expect(Object.keys(metrics.timing_by_lane)).toEqual(['1', '2'])
        expect(metrics.timing_by_lane['1']).toMatchObject({ notes: 3, mean_us: 767 })
        expect(metrics.timing_by_lane['2']).toMatchObject({ notes: 2, mean_us: 1500 })
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

type MetricsOptions = Partial<Omit<Parameters<typeof computePlayMetrics>[0], 'notes'>>

const metricsWith = (notes: JudgedPlayNote[], options: MetricsOptions = {}) =>
    computePlayMetrics({ notes, tempo: [], initBpm: BPM, emptyPoorTimesUs: [], lengthUs: LENGTH_US, gaugeDropsMs: [], ...options })

const TENTH = 100_000
const EMPTY_CELL = { notes: 0, ex_score: 0, bp: 0, fast: 0, slow: 0 }

describe('국소 NPS 칸과 최고 밀도(2.2.1)', () => {
    test('2초 창 NPS 의 정수 칸에 판정 받은 노트를 더하고, sustained·burst 문맥은 칸 안에 따로 둔다', () => {
        const stream = Array.from({ length: 201 }, (_, index) =>
            played(index * TENTH, index % 7, index === 0 ? JUDGE_STATE.GREAT : JUDGE_STATE.PGREAT, -500),
        )
        const metrics = metricsWith(stream.slice(0, 200), { lengthUs: 20 * SECOND })
        expect(Object.keys(metrics.by_nps)).toEqual(['10'])
        expect(metrics.by_nps['10']).toEqual({
            notes: 200,
            ex_score: 399,
            bp: 0,
            fast: 0,
            slow: 1,
            burst: EMPTY_CELL,
            sustained: { notes: 200, ex_score: 399, bp: 0, fast: 0, slow: 1 },
        })
        expect(metrics.peak_nps).toEqual({ '500': 10, '1000': 10, '2000': 10, '4000': 10, '8000': 10 })
        expect(metrics.peak_rps_2s).toBe(10)
    })

    test('짧게 몰린 노트는 burst 이고, 도달 못한 노트는 밀도에는 들지만 칸에는 들지 않는다', () => {
        const metrics = metricsWith([
            played(20 * SECOND, 0, JUDGE_STATE.PGREAT),
            played(20 * SECOND, 3, JUDGE_STATE.BAD, 25_000),
            played(20 * SECOND + TENTH, 1, JUDGE_STATE.PGREAT),
            played(20 * SECOND + 2 * TENTH, 2, JUDGE_STATE.PGREAT),
            played(20 * SECOND + 3 * TENTH, 4, JUDGE_STATE.NONE),
            played(30 * SECOND, 5, JUDGE_STATE.NONE),
        ])
        expect(metrics.by_nps).toEqual({
            '2': { notes: 4, ex_score: 6, bp: 1, fast: 1, slow: 0, burst: { notes: 4, ex_score: 6, bp: 1, fast: 1, slow: 0 }, sustained: EMPTY_CELL },
        })
        expect(metrics.peak_nps['500']).toBe(10)
        expect(metrics.peak_rps_2s).toBe(2)
    })
})

const CHORDS = [
    played(SECOND, 0, JUDGE_STATE.PGREAT, 1000),
    played(SECOND, 1, JUDGE_STATE.BAD, -20_000),
    played(5 * SECOND, 2, JUDGE_STATE.GREAT, -3000),
    played(5 * SECOND, 3, JUDGE_STATE.GOOD, 5000),
    played(10 * SECOND, 4, JUDGE_STATE.POOR),
]

describe('태그·레인·손별 오차 모멘트(2.2.2, 2.2.3)', () => {
    test('태그별 모멘트는 PGREAT~BAD 의 개수·합·제곱합(ms²)·FAST/SLOW 개수와 합이고 POOR 는 뺀다', () => {
        const metrics = metricsWith(CHORDS)
        expect(Object.keys(metrics.timing_by_tag)).toEqual(Object.keys(metrics.by_tag))
        expect(metrics.timing_by_tag.chord_2).toEqual({
            n: 4,
            sum_us: -17_000,
            sumsq_ms2: 435,
            fast_n: 2,
            fast_sum_us: 6000,
            slow_n: 2,
            slow_sum_us: -23_000,
        })
    })

    test('레인 오차에는 기존 평균 옆에 PGREAT~BAD 모멘트가 붙고, 레인별 BP 는 판정 받은 노트 기준이다', () => {
        const metrics = metricsWith(CHORDS)
        expect(metrics.timing_by_lane).toEqual({
            '1': { notes: 1, mean_us: 1000, moments: { n: 1, sum_us: 1000, sumsq_ms2: 1, fast_n: 1, fast_sum_us: 1000, slow_n: 0, slow_sum_us: 0 } },
            '3': {
                notes: 1,
                mean_us: -3000,
                moments: { n: 1, sum_us: -3000, sumsq_ms2: 9, fast_n: 0, fast_sum_us: 0, slow_n: 1, slow_sum_us: -3000 },
            },
        })
        expect(metrics.lane_bp).toEqual({
            '1': { notes: 1, bp: 0 },
            '2': { notes: 1, bp: 1 },
            '3': { notes: 1, bp: 0 },
            '4': { notes: 1, bp: 0 },
            '5': { notes: 1, bp: 1 },
        })
    })

    test('손은 1~3 왼손, 4 공용, 5~7 오른손, 스크래치 따로이고 노트가 있는 손만 남는다', () => {
        const metrics = metricsWith(CHORDS)
        expect(metrics.by_hand).toEqual({
            left: { notes: 3, ex_score: 3, bp: 1, fast: 0, slow: 2 },
            shared: { notes: 1, ex_score: 0, bp: 0, fast: 1, slow: 0 },
            right: { notes: 1, ex_score: 0, bp: 1, fast: 0, slow: 0 },
        })
        expect(metrics.timing_by_hand.left).toEqual({
            n: 3,
            sum_us: -22_000,
            sumsq_ms2: 410,
            fast_n: 1,
            fast_sum_us: 1000,
            slow_n: 2,
            slow_sum_us: -23_000,
        })
        expect(metrics.timing_by_hand.right).toEqual({ n: 0, sum_us: 0, sumsq_ms2: 0, fast_n: 0, fast_sum_us: 0, slow_n: 0, slow_sum_us: 0 })
        expect(metricsWith([played(SECOND, 7, JUDGE_STATE.GREAT, 100)]).by_hand).toEqual({
            scratch: { notes: 1, ex_score: 1, bp: 0, fast: 1, slow: 0 },
        })
    })
})

describe('게이지 급락 문맥(2.2.5)', () => {
    const notes = [
        played(1.5 * SECOND, 6, JUDGE_STATE.PGREAT),
        played(2 * SECOND, 0, JUDGE_STATE.BAD, -30_000),
        played(3 * SECOND, 1, JUDGE_STATE.POOR),
        played(3 * SECOND, 2, JUDGE_STATE.PGREAT),
        played(4 * SECOND, 4, JUDGE_STATE.GREAT, 2000),
        played(4.5 * SECOND, 5, JUDGE_STATE.PGREAT),
    ]
    const metrics = metricsWith(notes, {
        gaugeDropsMs: [
            { from_ms: 3000, to_ms: 4000 },
            { from_ms: 4500, to_ms: 5000 },
        ],
    })

    test('꺾임마다 1초 앞부터 꺾임 끝까지의 판정 노트 수·BP·평균 국소 NPS(마지막 노트에서 자른 창)·상위 태그를 남긴다', () => {
        expect(metrics.gauge_drop_contexts).toEqual([
            {
                from_ms: 3000,
                to_ms: 4000,
                chart_notes: 4,
                bp: 2,
                mean_nps: 1.7,
                top_tags: [
                    { tag: 'dense', notes: 3 },
                    { tag: 'chord_2', notes: 2 },
                ],
            },
            { from_ms: 4500, to_ms: 5000, chart_notes: 2, bp: 0, mean_nps: 2.3, top_tags: [{ tag: 'dense', notes: 2 }] },
        ])
    })

    test('모든 꺾임 창의 태그 합과 노트 수는 겹치는 노트를 한 번만 센다', () => {
        expect(metrics.gauge_drop_notes).toBe(5)
        expect(metrics.gauge_drop_tag_totals).toEqual({ chord_2: 2, dense: 4 })
        expect(metricsWith(notes)).toMatchObject({ gauge_drop_contexts: [], gauge_drop_tag_totals: {}, gauge_drop_notes: 0 })
    })

    test('판의 태그 조합과 단독 칸도 같은 노트에서 센다', () => {
        expect(metrics.by_combo).toEqual({ 'chord_2+dense': { notes: 2, ex_score: 2, bp: 1, fast: 0, slow: 0 } })
        expect(metrics.by_tag_solo).toEqual({ dense: { notes: 2, ex_score: 3, bp: 0, fast: 1, slow: 0 } })
    })
})

describe('BPM 띠와 소프란 국면(2.2.7)', () => {
    const tempo = [
        { tUs: 10 * SECOND, bpm: 200, stopUs: 0, scroll: 1 },
        { tUs: 14 * SECOND, bpm: 150, stopUs: 0, scroll: 1 },
    ]
    const notes = [
        played(5 * SECOND, 0, JUDGE_STATE.PGREAT),
        played(9 * SECOND, 1, JUDGE_STATE.PGREAT),
        played(10.5 * SECOND, 2, JUDGE_STATE.PGREAT),
        played(13 * SECOND, 3, JUDGE_STATE.GREAT, 2000),
        played(14.5 * SECOND, 4, JUDGE_STATE.PGREAT),
        played(17 * SECOND, 5, JUDGE_STATE.GOOD, -4000),
        played(21 * SECOND, 6, JUDGE_STATE.PGREAT),
    ]
    const metrics = metricsWith(notes, { tempo })

    test('노트 시각의 BPM 으로 다섯 띠에 나눈다', () => {
        expect(metrics.by_bpm_band).toEqual({
            '150-179': { notes: 5, ex_score: 8, bp: 0, fast: 0, slow: 1 },
            '180-209': { notes: 2, ex_score: 3, bp: 0, fast: 1, slow: 0 },
        })
        expect(Object.keys(metricsWith([played(SECOND, 0, JUDGE_STATE.PGREAT)], { initBpm: 119.5 }).by_bpm_band)).toEqual(['<120'])
        expect(Object.keys(metricsWith([played(SECOND, 0, JUDGE_STATE.PGREAT)], { initBpm: 210 }).by_bpm_band)).toEqual(['210+'])
    })

    test('사건 앞 2초·사건부터 정지+2초·그 뒤 6초까지를 나누고, 두 사건에 걸친 노트는 가까운 사건 하나에만 든다', () => {
        expect(metrics.soflan_phase).toEqual({
            events: 2,
            before: { notes: 2, ex_score: 3, bp: 0, fast: 1, slow: 0 },
            after: { notes: 2, ex_score: 4, bp: 0, fast: 0, slow: 0 },
            settled: { notes: 1, ex_score: 0, bp: 0, fast: 0, slow: 1 },
        })
    })

    test('정지는 after 국면을 정지 길이만큼 늘리고, 스크롤 속도가 바뀐 점도 사건이다', () => {
        const stopped = metricsWith([played(32.5 * SECOND, 0, JUDGE_STATE.PGREAT), played(36 * SECOND, 1, JUDGE_STATE.GREAT, 10)], {
            tempo: [{ tUs: 30 * SECOND, bpm: BPM, stopUs: SECOND, scroll: 1 }],
        })
        expect(stopped.soflan_phase).toMatchObject({ events: 1, after: { notes: 1 }, settled: { notes: 1 }, before: EMPTY_CELL })
        const scrolled = metricsWith([], {
            tempo: [
                { tUs: 0, bpm: BPM, stopUs: 0, scroll: 1 },
                { tUs: 20 * SECOND, bpm: BPM, stopUs: 0, scroll: 0.5 },
                { tUs: 25 * SECOND, bpm: BPM, stopUs: 0, scroll: 1 },
            ],
        })
        expect(scrolled.soflan_phase.events).toBe(2)
    })
})

describe('훈련곡 구간(2.2.8)', () => {
    test('구간이 없으면 null, 있으면 구간마다 시각 범위 안 판정 객체 수·셀·모멘트를 남긴다', () => {
        const notes = [
            played(0.5 * SECOND, 0, JUDGE_STATE.PGREAT, 1000),
            played(1.5 * SECOND, 1, JUDGE_STATE.GREAT, -2000),
            played(2 * SECOND, 2, JUDGE_STATE.BAD, 30_000),
            played(3 * SECOND, 3, JUDGE_STATE.NONE),
        ]
        expect(metricsWith(notes).training_sections).toBeNull()
        const sections = [
            { index: 0, kind: 'trill', focus_key: 'tag:trill', from_us: 0, to_us: 2 * SECOND },
            { index: 1, kind: 'jack', focus_key: 'tag:jack', from_us: 2 * SECOND, to_us: 4 * SECOND },
        ]
        expect(metricsWith(notes, { trainingSections: sections }).training_sections).toEqual([
            {
                index: 0,
                kind: 'trill',
                focus_key: 'tag:trill',
                from_ms: 0,
                to_ms: 2000,
                chart_notes: 2,
                notes: 2,
                ex_score: 3,
                bp: 0,
                fast: 0,
                slow: 1,
                moments: { n: 2, sum_us: -1000, sumsq_ms2: 5, fast_n: 1, fast_sum_us: 1000, slow_n: 1, slow_sum_us: -2000 },
            },
            {
                index: 1,
                kind: 'jack',
                focus_key: 'tag:jack',
                from_ms: 2000,
                to_ms: 4000,
                chart_notes: 2,
                notes: 1,
                ex_score: 0,
                bp: 1,
                fast: 1,
                slow: 0,
                moments: { n: 1, sum_us: 30_000, sumsq_ms2: 900, fast_n: 1, fast_sum_us: 30_000, slow_n: 0, slow_sum_us: 0 },
            },
        ])
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

    test('끝까지 친 판은 게이지마다 끝값·클리어 여부·바닥에 닿았는지를 읽기만 한다', () => {
        expect(
            summarizeGaugeCurves(
                [
                    { ...curve, values: [20, 50, 82] },
                    { ...curve, values: [100, 0, 0] },
                ],
                FINISHED_RUN,
            ),
        ).toEqual([
            { end: 82, cleared: true, reached_min: false, outcome: GAUGE_OUTCOME.CLEARED },
            { end: 0, cleared: false, reached_min: true, outcome: GAUGE_OUTCOME.FAILED },
        ])
    })
})

const FINISHED_RUN = { end: RUN_END.FINISHED, failedAtMs: null, selected: 0 } as const

describe('게이지 곡선 — 클리어 판정', () => {
    const groove = { intervalMs: 500, min: 2, max: 100, border: 80 }
    const survival = { intervalMs: 500, min: 0, max: 100, border: 0 }

    test('생존 게이지(최소 0, border 0)는 끝값이 0 보다 크면 클리어, 0 이면 아니다', () => {
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 40, 3.2] }], FINISHED_RUN)).toEqual([
            { end: 3.2, cleared: true, reached_min: false, outcome: GAUGE_OUTCOME.CLEARED },
        ])
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 40, 0, 0] }], FINISHED_RUN)).toEqual([
            { end: 0, cleared: false, reached_min: true, outcome: GAUGE_OUTCOME.FAILED },
        ])
    })

    test('그 밖의 게이지는 끝값이 border 이상일 때만 클리어다', () => {
        expect(
            summarizeGaugeCurves(
                [
                    { ...groove, values: [20, 80] },
                    { ...groove, values: [20, 79.99] },
                ],
                FINISHED_RUN,
            ).map((summary) => summary.cleared),
        ).toEqual([true, false])
    })

    test('Esc·종료로 남긴 판은 FAILED 로 기록되므로 끝값이 border 위여도 아홉 게이지 모두 클리어가 아니다', () => {
        const left = { end: RUN_END.LEFT, failedAtMs: null, selected: 0 } as const
        expect(
            summarizeGaugeCurves(
                [
                    { ...groove, values: [20, 90] },
                    { ...survival, values: [100, 100] },
                ],
                left,
            ),
        ).toEqual([
            { end: 90, cleared: false, reached_min: false, outcome: GAUGE_OUTCOME.FAILED },
            { end: 100, cleared: false, reached_min: false, outcome: GAUGE_OUTCOME.FAILED },
        ])
    })

    test('게이지가 빈 판은 실패 시점 뒤를 클라이언트가 0 으로 채우므로, 다른 게이지는 실패 시점까지만 읽고 그 뒤는 모른다', () => {
        const failedAtMs = 1000
        const emptied = { end: RUN_END.GAUGE_EMPTY, failedAtMs, selected: 1 } as const
        const padded = (live: number[]) => [...live, 0, 0, 0]
        expect(
            summarizeGaugeCurves(
                [
                    { ...groove, values: padded([20, 60, 85]) },
                    { ...survival, values: padded([100, 30, 0]) },
                    { ...survival, values: padded([100, 0, 0]) },
                    { ...survival, values: padded([100, 70, 40]) },
                ],
                emptied,
            ),
        ).toEqual([
            { end: 85, cleared: null, reached_min: false, outcome: GAUGE_OUTCOME.UNKNOWN },
            { end: 0, cleared: false, reached_min: true, outcome: GAUGE_OUTCOME.FAILED },
            { end: 0, cleared: false, reached_min: true, outcome: GAUGE_OUTCOME.FAILED },
            { end: 40, cleared: null, reached_min: false, outcome: GAUGE_OUTCOME.UNKNOWN },
        ])
    })

    test('실패 시점의 표본 수는 0 으로 채운 꼬리보다 짧은 곡선에서도 곡선 길이를 넘지 않는다', () => {
        const emptied = { end: RUN_END.GAUGE_EMPTY, failedAtMs: 60_000, selected: 0 } as const
        expect(summarizeGaugeCurves([{ ...survival, values: [100, 0] }], emptied)).toEqual([
            { end: 0, cleared: false, reached_min: true, outcome: GAUGE_OUTCOME.FAILED },
        ])
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
