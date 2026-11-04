import { describe, expect, test } from 'bun:test'
import type { TraceUploadInput } from '@server/dto/trace'
import { ANALYSIS_VERSION, PLAY_ANALYSIS_VERSION } from '@server/service/domain/analysis/analysis-constants'
import { RUN_END } from '@server/service/domain/analysis/play-metrics'
import { PATTERN_JOIN, analyzeTrace, joinPattern } from '@server/service/domain/trace/trace-analysis'
import { makePatternDocument, type FixtureNote } from '../../support/chart-pattern-fixture'
import { readContractTrace } from '../../support/trace-fixture'

const NORMAL = readContractTrace('trace-normal.json')
const RANDOM = readContractTrace('trace-random.json')
const SCRATCH_AUTO = readContractTrace('trace-scratch-auto.json')
const ABANDONED = readContractTrace('trace-abandoned.json')
const SCRATCH_LANE = 7
const LONG_END_KIND = 2
const MINE_KIND = 3
const SHIFT_US = 1

/** The contract chart's own layout, every note normal: the trace of an OFF run holds exactly these lanes and times. */
const contractPattern = (extra: readonly FixtureNote[] = []) =>
    makePatternDocument(
        [...NORMAL.notes.time_us.map((timeUs, index): FixtureNote => [timeUs, NORMAL.notes.lane[index], NORMAL.notes.kind[index]]), ...extra],
        { md5: NORMAL.chart.md5, sha256: NORMAL.chart.sha256 },
    )

const analyze = (trace: TraceUploadInput, pattern: ReturnType<typeof contractPattern> | null = null) =>
    analyzeTrace({ trace, chartSha256: trace.chart.sha256, pattern })

const SECOND_US = 1_000_000
const MICROS_PER_MILLI = 1000
const SYNTHETIC_NOTES = 60_000
/** Plan 2026-11-04 2.4 expects 15 to 30 KB a play; a 60,000-note run with a dozen gauge drops stays under 40 KB. */
const SUMMARY_BYTES_BUDGET = 40 * 1024
const SIXTEENTH_US = 100_000
const MEASURE_SIXTEENTHS = 16
const SCRATCH = 7
const LCG = { MULTIPLIER: 1_103_515_245, INCREMENT: 12_345, MODULUS: 2_147_483_648, SEED: 20_261_104 } as const
const GRADE_SHARES = [
    [0.55, 1],
    [0.85, 2],
    [0.93, 3],
    [0.97, 4],
    [1, 5],
] as const
const ERROR_SPREAD_US = 40_000
const GAUGE_INTERVAL_MS = 500
const GAUGE_DROPS = 12
const GAUGE_DROP_SAMPLES = 4
const GAUGE_DROP_STEP = 10
const GAUGE_FULL = 100
const SCROLL_EVENT_EVERY_US = 30 * SECOND_US
const SLOW_SCROLL = 0.8

type SyntheticNote = { timeUs: number; lane: number; kind: number }

/** One measure of each motif at 150 BPM: a trill, a stair, triple chords, a jack, scratch with keys, and keys under a held LN. */
const MOTIFS: readonly ((startUs: number) => SyntheticNote[])[] = [
    (startUs) => Array.from({ length: MEASURE_SIXTEENTHS }, (_, step) => ({ timeUs: startUs + step * SIXTEENTH_US, lane: 1 + (step % 2), kind: 0 })),
    (startUs) => Array.from({ length: MEASURE_SIXTEENTHS }, (_, step) => ({ timeUs: startUs + step * SIXTEENTH_US, lane: step % 7, kind: 0 })),
    (startUs) =>
        Array.from({ length: MEASURE_SIXTEENTHS / 2 }, (_, step) =>
            [0, 2, 4].map((lane) => ({ timeUs: startUs + 2 * step * SIXTEENTH_US, lane: lane + (step % 2), kind: 0 })),
        ).flat(),
    (startUs) => Array.from({ length: MEASURE_SIXTEENTHS / 2 }, (_, step) => ({ timeUs: startUs + 2 * step * SIXTEENTH_US, lane: 4, kind: 0 })),
    (startUs) =>
        Array.from({ length: MEASURE_SIXTEENTHS }, (_, step) => ({
            timeUs: startUs + step * SIXTEENTH_US,
            lane: step % 2 === 0 ? SCRATCH : 3,
            kind: 0,
        })),
    (startUs) => [
        { timeUs: startUs, lane: 0, kind: 1 },
        ...Array.from({ length: MEASURE_SIXTEENTHS - 2 }, (_, step) => ({
            timeUs: startUs + (step + 1) * SIXTEENTH_US,
            lane: 2 + (step % 5),
            kind: 0,
        })),
        { timeUs: startUs + (MEASURE_SIXTEENTHS - 1) * SIXTEENTH_US, lane: 0, kind: 2 },
    ],
]

/** A long run of the normal contract trace's shape whose notes cycle through MOTIFS, judged by a fixed-seed generator. */
const syntheticTrace = (count: number): TraceUploadInput => {
    let state: number = LCG.SEED
    const next = () => {
        state = (state * LCG.MULTIPLIER + LCG.INCREMENT) % LCG.MODULUS
        return state / LCG.MODULUS
    }
    const notes: SyntheticNote[] = []
    for (let measure = 0; notes.length < count; measure += 1) {
        notes.push(...MOTIFS[measure % MOTIFS.length]((measure + 1) * MEASURE_SIXTEENTHS * SIXTEENTH_US))
    }
    const ordered = notes.toSorted((a, b) => a.timeUs - b.timeUs || a.lane - b.lane)
    const gradeOf = (roll: number) => GRADE_SHARES.find(([share]) => roll < share)?.[1] ?? 1
    const states = ordered.map((note) => (note.kind === 2 ? 0 : gradeOf(next())))
    const lastMs = Math.ceil((ordered.at(-1)?.timeUs ?? 0) / MICROS_PER_MILLI)
    const samples = Math.floor(lastMs / GAUGE_INTERVAL_MS) + 1
    const dropEvery = Math.floor(samples / (GAUGE_DROPS + 1))
    const gaugeAt = (index: number) => {
        const intoDrop = index % dropEvery
        return index >= dropEvery && intoDrop < GAUGE_DROP_SAMPLES ? GAUGE_FULL - (intoDrop + 1) * GAUGE_DROP_STEP : GAUGE_FULL
    }
    const curve = Array.from({ length: samples }, (_, index) => gaugeAt(index))
    return {
        ...NORMAL,
        notes: {
            time_us: ordered.map((note) => note.timeUs),
            lane: ordered.map((note) => note.lane),
            kind: ordered.map((note) => note.kind),
            state: states,
            err_us: states.map((grade) => (grade === 0 || grade === 5 ? 0 : Math.round((next() - 0.5) * grade * ERROR_SPREAD_US))),
        },
        tempo: Array.from({ length: Math.floor((lastMs * MICROS_PER_MILLI) / SCROLL_EVENT_EVERY_US) + 1 }, (_, index) => ({
            t_us: index * SCROLL_EVENT_EVERY_US,
            bpm: 150,
            stop_us: 0,
            scroll: index % 2 === 0 ? 1 : SLOW_SCROLL,
        })),
        gauge: { ...NORMAL.gauge, interval_ms: GAUGE_INTERVAL_MS, series: NORMAL.gauge.series.map(() => curve) },
        play_time_ms: lastMs,
    }
}

describe('joinPattern — trace 와 곡별 패턴의 정합(기획 3.4)', () => {
    test('패턴이 없으면 absent, sha256 이 다르면 mismatch 다', () => {
        expect(joinPattern({ trace: NORMAL, chartSha256: NORMAL.chart.sha256, pattern: null })).toBe(PATTERN_JOIN.ABSENT)
        expect(joinPattern({ trace: NORMAL, chartSha256: 'c'.repeat(64), pattern: contractPattern() })).toBe(PATTERN_JOIN.MISMATCH)
    })

    test('OFF·RANDOM 판의 노트 시각은 원본 배치의 시각과 같다(지뢰는 빼고 본다)', () => {
        const pattern = contractPattern([[NORMAL.notes.time_us[0], 1, MINE_KIND]])
        expect(joinPattern({ trace: NORMAL, chartSha256: NORMAL.chart.sha256, pattern })).toBe(PATTERN_JOIN.MATCHED)
        expect(joinPattern({ trace: RANDOM, chartSha256: RANDOM.chart.sha256, pattern })).toBe(PATTERN_JOIN.MATCHED)
    })

    test('SCRATCH AUTO 판은 자동 레인의 원본 노트를 빼고 맞춘다', () => {
        expect(SCRATCH_AUTO.auto_lanes[SCRATCH_LANE]).toBe(true)
        expect(joinPattern({ trace: SCRATCH_AUTO, chartSha256: SCRATCH_AUTO.chart.sha256, pattern: contractPattern() })).toBe(PATTERN_JOIN.MATCHED)
        expect(joinPattern({ trace: { ...SCRATCH_AUTO, auto_lanes: [] }, chartSha256: SCRATCH_AUTO.chart.sha256, pattern: contractPattern() })).toBe(
            PATTERN_JOIN.MISMATCH,
        )
    })

    test('ALL-SCRATCH 처럼 노트를 스크래치 레인으로 옮기는 옵션과 SCRATCH AUTO 가 함께인 판은 자동으로 친 노트를 원본 레인으로 가릴 수 없어 unsupported 다', () => {
        const scratchIndex = SCRATCH_AUTO.notes.time_us.length
        const pattern = contractPattern()
        const patternTimes = pattern.notes.time_us.filter((_, index) => pattern.notes.lane[index] !== SCRATCH_LANE)
        const keyTimes = [...patternTimes.slice(1), ...pattern.notes.time_us.filter((_, index) => pattern.notes.lane[index] === SCRATCH_LANE)]
        const moved = { ...SCRATCH_AUTO, random: 'AllScratch' as const, notes: { ...SCRATCH_AUTO.notes, time_us: keyTimes.slice(0, scratchIndex) } }
        expect(joinPattern({ trace: moved, chartSha256: moved.chart.sha256, pattern })).toBe(PATTERN_JOIN.UNSUPPORTED)
        const foreign = {
            ...moved,
            notes: { ...moved.notes, time_us: moved.notes.time_us.map((time, index) => (index === 0 ? time + SHIFT_US : time)) },
        }
        expect(joinPattern({ trace: foreign, chartSha256: moved.chart.sha256, pattern })).toBe(PATTERN_JOIN.MISMATCH)
        expect(joinPattern({ trace: { ...SCRATCH_AUTO, random: 'Random' }, chartSha256: SCRATCH_AUTO.chart.sha256, pattern })).toBe(
            PATTERN_JOIN.MATCHED,
        )
    })

    test('시각이 1 µs 라도 다르거나 노트가 하나 더 있으면 mismatch 다', () => {
        const shifted = {
            ...NORMAL,
            notes: { ...NORMAL.notes, time_us: NORMAL.notes.time_us.map((time, index) => (index === 0 ? time + SHIFT_US : time)) },
        }
        expect(joinPattern({ trace: shifted, chartSha256: NORMAL.chart.sha256, pattern: contractPattern() })).toBe(PATTERN_JOIN.MISMATCH)
        const extraEnd: FixtureNote = [NORMAL.notes.time_us[1], 3, LONG_END_KIND]
        expect(joinPattern({ trace: NORMAL, chartSha256: NORMAL.chart.sha256, pattern: contractPattern([extraEnd]) })).toBe(PATTERN_JOIN.MISMATCH)
    })
})

describe('analyzeTrace — play_analysis.summary', () => {
    test('끝까지 친 판의 지표는 trace 의 판정 그대로이고, 녹색 숫자는 추정이 아니다', () => {
        const { summary, patternJoin, patternMd5 } = analyze(NORMAL, contractPattern())
        expect(summary).toMatchObject({
            analysis_version: PLAY_ANALYSIS_VERSION,
            mode: 'BEAT_7K',
            scratch_auto: false,
            run_end: RUN_END.FINISHED,
            pattern: { join: PATTERN_JOIN.MATCHED, md5: NORMAL.chart.md5 },
            speed: {
                start: { green: NORMAL.speed.start.green, lane_ms: NORMAL.speed.start.green_ms, is_estimated: false },
                end: { green: NORMAL.speed.end.green, lane_ms: NORMAL.speed.end.green_ms, is_estimated: false },
                changes: 0,
                changed_fields: [],
            },
        })
        expect(summary.metrics?.overall).toMatchObject({ pgreat: 15, great: 11, good: 3, bad: 0, poor: 3, notes: 32 })
        expect(summary.metrics?.segments).toHaveLength(32)
        expect(summary.gauge.curves).toHaveLength(NORMAL.gauge.series.length)
        expect([patternJoin, patternMd5]).toEqual([PATTERN_JOIN.MATCHED, NORMAL.chart.md5])
    })

    test('패턴이 맞지 않으면 곡별 특징을 잇지 않는다(md5 없음)', () => {
        const { summary, patternMd5 } = analyze(NORMAL, { ...contractPattern(), sha256: 'd'.repeat(64) })
        expect(summary.pattern).toEqual({ join: PATTERN_JOIN.MISMATCH, md5: null })
        expect(patternMd5).toBeNull()
    })

    test('Esc 로 남긴 판은 left 이고 도달하지 못한 노트는 지표에 넣지 않는다', () => {
        const { summary } = analyze(ABANDONED)
        expect(summary.run_end).toBe(RUN_END.LEFT)
        expect(summary.metrics?.overall.notes).toBe(7)
    })

    test('게이지가 빈 판은 gauge_empty 이고, 친 게이지는 클리어가 아니다', () => {
        const failedAtMs = 2500
        const { summary } = analyze({ ...ABANDONED, failed_at_ms: failedAtMs })
        expect(summary.run_end).toBe(RUN_END.GAUGE_EMPTY)
        expect(summary.gauge.failed_at_ms).toBe(failedAtMs)
        expect(summary.gauge.curves[ABANDONED.gauge.selected].cleared).toBe(false)
    })

    test('Esc 로 남긴 판은 FAILED 로 기록되므로 끝값과 상관없이 아홉 게이지 모두 클리어가 아니다', () => {
        const { summary } = analyze(ABANDONED)
        expect(summary.gauge.curves).toHaveLength(ABANDONED.gauge.series.length)
        expect(summary.gauge.curves.every((curve) => curve.cleared === false)).toBe(true)
    })

    test('게이지가 빈 판은 실패 시점 뒤 0 으로 채운 값을 읽지 않고 다른 게이지는 알 수 없음으로 둔다', () => {
        const failedAtMs = 1000
        const liveSamples = failedAtMs / ABANDONED.gauge.interval_ms + 1
        const series = ABANDONED.gauge.series.map((values) => values.map((value, index) => (index < liveSamples ? value : 0)))
        const { summary } = analyze({ ...ABANDONED, failed_at_ms: failedAtMs, gauge: { ...ABANDONED.gauge, series } })
        const curves = summary.gauge.curves
        expect(curves[ABANDONED.gauge.selected]).toMatchObject({ cleared: false })
        const others = curves.filter((_, index) => index !== ABANDONED.gauge.selected)
        expect(others.every((curve) => curve.cleared === null && curve.reached_min === false)).toBe(true)
        expect(curves[0].end).toBe(ABANDONED.gauge.series[0][liveSamples - 1])
        const segments = summary.metrics?.segments ?? []
        expect(summary.gauge.segment_values.filter((_, index) => (segments[index]?.to_ms ?? 0) > failedAtMs).every((value) => value === null)).toBe(
            true,
        )
        expect(summary.gauge.segment_values[0]).not.toBeNull()
    })

    test('Esc 로 나간 판은 나간 시점 뒤 구간의 게이지를 마지막 표본으로 채우지 않고 알 수 없음으로 둔다', () => {
        const { summary } = analyze(ABANDONED)
        const lastSampleMs = (ABANDONED.gauge.series[0].length - 1) * ABANDONED.gauge.interval_ms
        const segments = summary.metrics?.segments ?? []
        const known = summary.gauge.segment_values.flatMap((value, index) => ((segments[index]?.to_ms ?? 0) <= lastSampleMs ? [value] : []))
        const unknown = summary.gauge.segment_values.flatMap((value, index) => ((segments[index]?.to_ms ?? 0) > lastSampleMs ? [value] : []))
        expect(known.length).toBeGreaterThan(0)
        expect(known.every((value) => value !== null)).toBe(true)
        expect(unknown.length).toBeGreaterThan(0)
        expect(unknown.every((value) => value === null)).toBe(true)
    })

    test('끝까지 친 판은 마지막 구간까지 게이지 값이 있다', () => {
        const { summary } = analyze(NORMAL)
        expect(summary.gauge.segment_values.every((value) => value !== null)).toBe(true)
    })

    test('요약에 곡별 분류 버전도 남긴다', () => {
        expect(analyze(NORMAL).summary.pattern_analysis_version).toBe(ANALYSIS_VERSION)
    })

    test('속도 변경 기록은 횟수와 바뀐 칸 이름만 남긴다', () => {
        const changes = [
            { t_ms: 1000, field: 'hispeed' as const, value: 2.5 },
            { t_ms: 1000, field: 'green' as const, value: 400 },
            { t_ms: 2000, field: 'hispeed' as const, value: 3 },
        ]
        const { summary } = analyze({ ...NORMAL, speed: { ...NORMAL.speed, changes } })
        expect(summary.speed).toMatchObject({ changes: 3, changed_fields: ['hispeed', 'green'] })
    })

    test('노트 순서가 섞여 와도 같은 지표를 낸다', () => {
        const order = NORMAL.notes.time_us.map((_, index) => index).toReversed()
        const pick = (column: readonly number[]) => order.map((index) => column[index])
        const shuffled = {
            ...NORMAL,
            notes: {
                time_us: pick(NORMAL.notes.time_us),
                lane: pick(NORMAL.notes.lane),
                kind: pick(NORMAL.notes.kind),
                state: pick(NORMAL.notes.state),
                err_us: pick(NORMAL.notes.err_us),
            },
        }
        expect(analyze(shuffled).summary.metrics).toEqual(analyze(NORMAL).summary.metrics)
    })

    test('7키 단일 플레이가 아닌 판은 레인 기반 지표 없이 게이지·속도만 남긴다', () => {
        const { summary } = analyze({ ...NORMAL, mode: 'BEAT_14K' })
        expect(summary.metrics).toBeNull()
        expect(summary.gauge.curves).toHaveLength(NORMAL.gauge.series.length)
    })
})

describe('analyzeTrace — 판 요약 v3(2.2, 2.4)', () => {
    test('버전은 판 요약 3, 곡별 분류 2 다', () => {
        expect([PLAY_ANALYSIS_VERSION, ANALYSIS_VERSION]).toEqual([3, 2])
        expect(analyze(NORMAL).summary.metrics?.analysis_version).toBe(2)
    })

    test('훈련곡 구간은 넘기지 않으면 null 이고, 넘기면 그 구간으로 나눈다', () => {
        expect(analyze(NORMAL).summary.metrics?.training_sections).toBeNull()
        const playEndUs = NORMAL.play_time_ms * MICROS_PER_MILLI
        const training = [{ index: 0, kind: 'stream', focus_key: 'tag:stream', from_us: 0, to_us: playEndUs }]
        const { summary } = analyzeTrace({ trace: NORMAL, chartSha256: NORMAL.chart.sha256, pattern: null, training })
        expect(summary.metrics?.training_sections).toMatchObject([
            { index: 0, kind: 'stream', focus_key: 'tag:stream', notes: summary.metrics?.overall.notes },
        ])
    })

    test('친 게이지의 꺾임을 먼저 구해 꺾임마다 차트 문맥을 남긴다', () => {
        const { summary } = analyze(NORMAL)
        expect(summary.metrics?.gauge_drop_contexts.map(({ from_ms, to_ms }) => ({ from_ms, to_ms }))).toEqual(
            summary.gauge.drops.map(({ from_ms, to_ms }) => ({ from_ms, to_ms })),
        )
        const series = NORMAL.gauge.series.map((values) => values.map((value, index) => (index === 10 ? value - 40 : value)))
        const dropped = analyze({ ...NORMAL, gauge: { ...NORMAL.gauge, series } }).summary
        expect(dropped.gauge.drops.length).toBeGreaterThan(0)
        expect(dropped.metrics?.gauge_drop_contexts).toHaveLength(dropped.gauge.drops.length)
    })

    test('소프란 사건 뒤 3초 안에 속도 설정을 바꾼 사건 수를 센다', () => {
        const tempo = [
            { t_us: 0, bpm: 150, stop_us: 0, scroll: 1 },
            { t_us: 2 * SECOND_US, bpm: 300, stop_us: 0, scroll: 1 },
            { t_us: 6 * SECOND_US, bpm: 150, stop_us: 0, scroll: 1 },
        ]
        const changes = [
            { t_ms: 4999, field: 'green' as const, value: 300 },
            { t_ms: 9000, field: 'green' as const, value: 600 },
        ]
        const { summary } = analyze({ ...NORMAL, tempo, speed: { ...NORMAL.speed, changes } })
        expect(summary.speed.soflan_adjust).toEqual({ events: 2, adjusted: 1 })
        expect(analyze(NORMAL).summary.speed.soflan_adjust).toEqual({ events: 0, adjusted: 0 })
    })

    test('6만 노트 합성 trace 의 요약은 SUMMARY_BYTES_BUDGET 안이고 같은 입력은 같은 바이트다', () => {
        const trace = syntheticTrace(SYNTHETIC_NOTES)
        const bytes = JSON.stringify(analyze(trace).summary)
        expect(trace.notes.time_us.length).toBeGreaterThanOrEqual(SYNTHETIC_NOTES)
        expect(Buffer.byteLength(bytes)).toBeLessThanOrEqual(SUMMARY_BYTES_BUDGET)
        expect(JSON.stringify(analyze(syntheticTrace(SYNTHETIC_NOTES)).summary)).toBe(bytes)
    })
})
