import { describe, expect, test } from 'bun:test'
import {
    TRAINING_PATTERN_KINDS,
    TRAINING_SHAPES,
    type AiTrainingPlanSection,
    type TrainingLaneHint,
    type TrainingPatternKind,
    type TrainingShape,
} from '@server/dto/training'
import { toPatternInput } from '@server/service/domain/analysis/chart-features'
import { localDensitiesOf } from '@server/service/domain/analysis/local-density'
import { classifyPattern, type PatternTag } from '@server/service/domain/analysis/pattern-tags'
import { TRAINING_NOTE_KINDS, writeTrainingBms, type TrainingNote } from '@server/service/domain/training/bms-writer'
import { buildTrainingPatternDocument } from '@server/service/domain/training/pattern-document'
import { correctTrainingPlan, trainingBoundsOf, trainingSpecInputOf, type TrainingSpecInput } from '@server/service/domain/training/spec'
import { synthesizeTraining } from '@server/service/domain/training/synthesize'
import { TRAINING_GRIDS, TRAINING_LENGTH_US, TRAINING_MEASURE_TICKS, TRAINING_TAIL_US } from '@server/service/domain/training/training-constants'

const NOTE_KINDS = TRAINING_NOTE_KINDS

const ID = `tc_${'c'.repeat(32)}`
const SEEDS = [1, 2, 3] as const
const BOUNDS = trainingBoundsOf({ limit_nps_ex: 12, max_seen_nps: 14 }, 'push')
const SCRATCH = 7
const LONG_END = 2
const LEVEL = 0.5
const FLOW_PARTS = 12

type SectionOptions = Partial<AiTrainingPlanSection> & { kind: TrainingPatternKind }

const specOf = (options: SectionOptions, seed: number, baseBpm = 160, shape: TrainingShape = 'flat', bounds = BOUNDS): TrainingSpecInput => {
    const sections = [0, 1, 2].map(() => ({
        focus_key: 'focus',
        weight: 1,
        lane_hint: 'all' as TrainingLaneHint,
        chord_size: 2,
        scratch_ratio: 0,
        ln_ratio: 0,
        bpm_ratio: 1,
        intent: '',
        ...options,
    }))
    const { spec, corrections } = correctTrainingPlan(
        { description: '', base_bpm: baseBpm, level: LEVEL, shape, sections, advice: '' },
        { allowed: { kinds: [], lane_hints: [], keys: ['focus'] }, bounds },
    )
    return { ...trainingSpecInputOf({ id: ID, userId: 'u', createdAt: 0, intensity: 'push', plan: spec, corrections }), seed }
}

const chartOf = (input: TrainingSpecInput) => {
    const { spec, notes } = synthesizeTraining(input)
    const written = writeTrainingBms(spec, notes)
    const document = buildTrainingPatternDocument({ spec, lines: written.lines, md5: written.md5, sha256: written.sha256 })
    return { spec, notes, written, document, tags: classifyPattern(toPatternInput(document)) }
}

type Chart = ReturnType<typeof chartOf>

/** The share of the judged notes (on `lanes`, every lane when not given) that carry `tag`. */
const shareOf = (chart: Chart, tag: PatternTag, lanes?: readonly number[]) => {
    const { kind, lane } = chart.document.notes
    const judged = chart.tags.filter((_, index) => kind[index] !== LONG_END && (lanes === undefined || lanes.includes(lane[index])))
    return judged.length === 0 ? 0 : judged.filter((tags) => tags.includes(tag)).length / judged.length
}

const KEYS = [0, 1, 2, 3, 4, 5, 6]

const rankOf = (sorted: readonly number[], share: number) => sorted[Math.max(0, Math.ceil(share * sorted.length) - 1)] ?? 0

/** The body's nps2 (the judged notes after the warm-up, as the analysis engine reads them) and the variation of its twelve parts. */
const bodyStatsOf = (chart: Chart) => {
    const { document, spec } = chart
    const notes = document.notes.time_us.map((timeUs, index) => ({
        timeUs,
        lane: document.notes.lane[index],
        kind: NOTE_KINDS[document.notes.kind[index]],
    }))
    const densities = localDensitiesOf(notes)
    const body = spec.sections.filter((section) => section.focus_key !== 'warmup')
    const fromUs = body[0].from_us
    const toUs = Math.min(TRAINING_LENGTH_US - TRAINING_TAIL_US, body[body.length - 1].to_us)
    const judged = notes.flatMap((note, index) =>
        note.kind !== 'ln_end' && note.timeUs >= fromUs ? [{ timeUs: note.timeUs, nps2: densities[index].nps2 }] : [],
    )
    const sorted = judged.map((note) => note.nps2).toSorted((a, b) => a - b)
    const partUs = (toUs - fromUs) / FLOW_PARTS
    const parts = Array.from(
        { length: FLOW_PARTS },
        (_, part) => judged.filter((note) => Math.floor((note.timeUs - fromUs) / partUs) === part).length,
    )
    const mean = parts.reduce((sum, count) => sum + count, 0) / FLOW_PARTS
    const variation = Math.sqrt(parts.reduce((sum, count) => sum + (count - mean) ** 2, 0) / FLOW_PARTS) / mean
    return { p50: rankOf(sorted, 0.5), p90: rankOf(sorted, 0.9), max: sorted.at(-1) ?? 0, variation }
}

const tickOf = (note: TrainingNote) => note.measure * TRAINING_MEASURE_TICKS + (note.num * TRAINING_MEASURE_TICKS) / note.den

describe('합성 결과가 분류기에서 의도한 태그를 갖는다 (시드 1·2·3)', () => {
    const cases: { kind: TrainingPatternKind; tag: PatternTag; min: number; lanes?: readonly number[]; options?: Partial<AiTrainingPlanSection> }[] =
        [
            { kind: 'stream', tag: 'stream', min: 0.9 },
            { kind: 'trill', tag: 'trill', min: 0.9 },
            { kind: 'stair', tag: 'stair', min: 0.9 },
            { kind: 'jack', tag: 'jack', min: 0.9 },
            { kind: 'chord_stream', tag: 'chord_2', min: 0.9 },
            { kind: 'chord_stream', tag: 'chord_3', min: 0.9, options: { chord_size: 3 } },
            { kind: 'chord_jack', tag: 'jack', min: 0.9 },
            { kind: 'scratch_solo', tag: 'scratch_solo', min: 0.9, lanes: [SCRATCH] },
            { kind: 'scratch_run', tag: 'scratch_run', min: 0.9, lanes: [SCRATCH] },
            { kind: 'scratch_mixed', tag: 'with_scratch', min: 0.8, lanes: KEYS },
            { kind: 'ln_hold', tag: 'ln_hold_hit', min: 0.7 },
            { kind: 'ln_release', tag: 'ln_release_then_hit', min: 0.45 },
            { kind: 'ln_release', tag: 'ln_head', min: 0.45 },
        ]
    for (const { kind, tag, min, lanes, options } of cases) {
        test(`${kind} 구간 노트의 ${min * 100}% 이상이 ${tag}${lanes ? ` (레인 ${lanes.join(',')})` : ''}`, () => {
            for (const seed of SEEDS) expect(shareOf(chartOf(specOf({ kind, ...options }, seed)), tag, lanes)).toBeGreaterThanOrEqual(min)
        })
    }

    test('scratch_mixed 의 스크래치는 모두 건반 행과 같은 칸이거나 16분 뒤다', () => {
        for (const seed of SEEDS) {
            const chart = chartOf(specOf({ kind: 'scratch_mixed' }, seed))
            const mixed = shareOf(chart, 'scratch_mixed', [SCRATCH]) + shareOf(chart, 'scratch_chord', [SCRATCH])
            expect(mixed).toBe(1)
        }
    })

    test('burst 는 구간마다 밀집 마디(짝수 번째)에 성긴 마디보다 노트가 두 배 넘게 많다', () => {
        for (const seed of SEEDS) {
            const { notes, spec } = chartOf(specOf({ kind: 'burst' }, seed))
            const offsetOf = (note: TrainingNote) => {
                const section = spec.sections.find(
                    (item) => note.measure >= item.measure_from && note.measure < item.measure_from + item.measure_count,
                )
                return section ? (note.measure - section.measure_from) % 2 : 0
            }
            const measures = Map.groupBy(notes, offsetOf)
            expect(measures.get(0)?.length ?? 0).toBeGreaterThan((measures.get(1)?.length ?? 0) * 2)
        }
    })
})

describe('합성 규칙', () => {
    const ALL_KINDS = TRAINING_PATTERN_KINDS

    test('같은 사양·시드면 바이트가 같고 시드가 다르면 다르다', () => {
        for (const kind of ALL_KINDS) {
            const first = chartOf(specOf({ kind }, 7)).written
            expect(Buffer.from(chartOf(specOf({ kind }, 7)).written.bytes).equals(Buffer.from(first.bytes))).toBe(true)
            if (kind !== 'rest') expect(chartOf(specOf({ kind }, 8)).written.md5).not.toBe(first.md5)
        }
    })

    test('모든 종류가 60초 안(꼬리 전)에 끝나고 채널 18·19 를 써서 7키로 읽힌다', () => {
        for (const kind of ALL_KINDS) {
            for (const seed of SEEDS) {
                const { document, written } = chartOf(specOf({ kind }, seed, 220))
                expect(document.notes.time_us.at(-1) ?? 0).toBeLessThan(TRAINING_LENGTH_US - TRAINING_TAIL_US)
                expect(written.lines.some((line) => line.channel === '18' || line.channel === '19')).toBe(true)
            }
        }
    })

    test('왼쪽 레인만 쓰는 곡에는 마지막 행에 7번 건반이 더해진다', () => {
        const { notes, written } = chartOf(specOf({ kind: 'stream', lane_hint: 'left' }, 1))
        const sevens = notes.filter((note) => note.lane === 6)
        expect(sevens).toHaveLength(1)
        const lastTick = Math.max(...notes.filter((note) => note.lane !== 6).map(tickOf))
        expect(tickOf(sevens[0])).toBe(lastTick)
        expect(written.lines.filter((line) => line.channel === '19')).toHaveLength(1)
    })

    test('한 레인의 같은 칸에 두 노트가 없고 LN 몸통 안에 같은 레인 노트가 없다', () => {
        for (const kind of ALL_KINDS) {
            for (const seed of SEEDS) {
                const { notes } = chartOf(specOf({ kind, ln_ratio: 0.4, scratch_ratio: 0.3 }, seed))
                for (const lane of [...KEYS, SCRATCH]) {
                    const onLane = notes.filter((note) => note.lane === lane).map((note) => ({ tick: tickOf(note), kind: note.kind }))
                    expect(new Set(onLane.map((note) => note.tick)).size).toBe(onLane.length)
                    onLane.forEach((note, index) => {
                        if (note.kind === 'ln_head') expect(onLane[index + 1]?.kind).toBe('ln_end')
                    })
                }
            }
        }
    })

    test('ln_ratio 와 scratch_ratio 는 LN 과 스크래치를 더하고, 스크래치·LN 종류에서는 0 으로 보정된다', () => {
        const mixed = chartOf(specOf({ kind: 'stream', ln_ratio: 0.5, scratch_ratio: 0.5 }, 1))
        expect(mixed.notes.some((note) => note.kind === 'ln_head')).toBe(true)
        expect(mixed.notes.some((note) => note.lane === SCRATCH)).toBe(true)
        const plain = chartOf(specOf({ kind: 'stream' }, 1))
        expect(plain.notes.some((note) => note.kind === 'ln_head' || note.lane === SCRATCH)).toBe(false)
        expect(specOf({ kind: 'ln_hold', ln_ratio: 0.5 }, 1).sections[0].ln_ratio).toBe(0)
    })

    test('trill 의 adjacent 는 이웃 건반, wide 는 3칸 이상 떨어진 건반 쌍이다', () => {
        for (const [hint, fits] of [
            ['adjacent', (gap: number) => gap === 1],
            ['wide', (gap: number) => gap >= 3],
        ] as const) {
            for (const seed of SEEDS) {
                const { notes, spec } = chartOf(specOf({ kind: 'trill', lane_hint: hint }, seed))
                const sectionStarts = new Set(spec.sections.map((section) => section.measure_from * TRAINING_MEASURE_TICKS))
                const runs = notes.reduce<TrainingNote[][]>((found, note) => {
                    const run = found.at(-1)
                    const last = run?.at(-1)
                    const continues =
                        last !== undefined && tickOf(note) - tickOf(last) <= TRAINING_MEASURE_TICKS / 16 && !sectionStarts.has(tickOf(note))
                    if (run && continues) return [...found.slice(0, -1), [...run, note]]
                    return [...found, [note]]
                }, [])
                const pairs = runs.filter((run) => run.length > 1).map((run) => [...new Set(run.map((note) => note.lane))])
                expect(pairs.length).toBeGreaterThan(0)
                expect(pairs.every((lanes) => lanes.length === 2 && fits(Math.abs(lanes[0] - lanes[1])))).toBe(true)
            }
        }
    })

    test('구간 필드와 bounds 를 채우고 PLAYLEVEL 은 본문 nps2 p90 의 반올림이다', () => {
        const chart = chartOf(specOf({ kind: 'chord_stream' }, 1))
        const { spec, document } = chart
        const rowTimes = new Set(document.notes.time_us.filter((_, index) => document.notes.kind[index] !== LONG_END))
        for (const section of spec.sections) {
            expect(section.to_us).toBeGreaterThan(section.from_us)
            expect(section.notes).toBeGreaterThan(0)
            expect((TRAINING_GRIDS as readonly number[]).includes(section.grid)).toBe(true)
            expect(section.actual_nps).toBeCloseTo((section.notes * 1_000_000) / (section.to_us - section.from_us), 1)
            const rows = [...rowTimes].filter((time) => time >= section.from_us && time < section.to_us).length
            expect(section.actual_rps).toBeCloseTo((rows * 1_000_000) / (section.to_us - section.from_us), 1)
            expect(section.actual_rps ?? 0).toBeLessThan(section.actual_nps)
        }
        expect(spec.play_level).toBe(Math.round(bodyStatsOf(chart).p90))
        expect(spec.bounds).toMatchObject({ limit_nps: 12, level: LEVEL, p90_target: 12.6, shape: 'flat' })
        expect(spec.bounds.mean_nps).toBeGreaterThan(0)
        expect(spec.sections[0].from_us).toBe(1_500_000)
    })

    test('구간마다 실제 NPS 가 목표(밀도 곡선의 구간 평균)의 0.93~1.07 이다', () => {
        for (const kind of TRAINING_PATTERN_KINDS.filter((item) => item !== 'rest')) {
            for (const seed of SEEDS) {
                for (const section of chartOf(specOf({ kind, chord_size: 3 }, seed)).spec.sections) {
                    const ratio = section.actual_nps / section.target_nps
                    expect(ratio, `${kind} ${seed} ${section.index}`).toBeGreaterThanOrEqual(0.93)
                    expect(ratio, `${kind} ${seed} ${section.index}`).toBeLessThanOrEqual(1.07)
                }
            }
        }
    })

    test('scratch_solo 는 밀도가 낮으면 떨어진 행에 건반을 두고, 스크래치는 여전히 독립 스크래치다', () => {
        const low = trainingBoundsOf({ limit_nps_ex: 6, max_seen_nps: null }, 'build')
        for (const seed of SEEDS) {
            const chart = chartOf(specOf({ kind: 'scratch_solo' }, seed, 160, 'flat', low))
            const keys = chart.notes.filter((note) => note.lane !== SCRATCH).length
            expect(keys).toBeGreaterThan(0)
            expect(keys).toBeLessThanOrEqual(chart.notes.filter((note) => note.lane === SCRATCH).length * 0.25 + 1)
            expect(shareOf(chart, 'scratch_solo', [SCRATCH])).toBeGreaterThanOrEqual(0.9)
        }
    })
})

describe('밀도 흐름 (설계 2026-11-04-training-difficulty 2절)', () => {
    test('본문 nps2 p90 은 bounds 의 목표 ±5% 안이고, max/p50 ≥ 1.25, 12구간 변동계수 0.12~0.35 이다', () => {
        for (const shape of TRAINING_SHAPES) {
            for (const kind of ['stream', 'chord_stream', 'trill', 'scratch_mixed', 'ln_hold'] as const) {
                for (const seed of SEEDS) {
                    const chart = chartOf(specOf({ kind, chord_size: 3 }, seed, 160, shape))
                    const stats = bodyStatsOf(chart)
                    const label = `${shape} ${kind} ${seed}`
                    expect(stats.p90 / chart.spec.bounds.p90_target, label).toBeGreaterThanOrEqual(0.95)
                    expect(stats.p90 / chart.spec.bounds.p90_target, label).toBeLessThanOrEqual(1.05)
                    expect(stats.max / stats.p50, label).toBeGreaterThanOrEqual(1.25)
                    expect(stats.variation, label).toBeGreaterThanOrEqual(0.12)
                    expect(stats.variation, label).toBeLessThanOrEqual(0.35)
                }
            }
        }
    })

    test('같은 시드·모양이면 밀도 곡선과 μ 가 같고, 모양이 다르면 구간 목표가 다르다', () => {
        const flat = synthesizeTraining(specOf({ kind: 'stream' }, 4, 160, 'flat'))
        expect(synthesizeTraining(specOf({ kind: 'stream' }, 4, 160, 'flat')).spec.bounds).toEqual(flat.spec.bounds)
        const rising = synthesizeTraining(specOf({ kind: 'stream' }, 4, 160, 'rising'))
        expect(rising.spec.sections.map((section) => section.target_nps)).not.toEqual(flat.spec.sections.map((section) => section.target_nps))
        expect(rising.spec.sections[0].target_nps).toBeLessThan(rising.spec.sections[2].target_nps)
    })
})

describe('합성 결과 재현', () => {
    test('합성 결과 사양을 다시 합성하면 같은 사양과 노트가 나온다', () => {
        const ALL_KINDS = TRAINING_PATTERN_KINDS
        for (const kind of ALL_KINDS) {
            const first = synthesizeTraining(specOf({ kind }, 5))
            expect(synthesizeTraining(first.spec)).toEqual(first)
        }
    })
})
