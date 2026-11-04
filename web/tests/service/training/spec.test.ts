import { describe, expect, test } from 'bun:test'
import { isAppError } from '@server/lib/error'
import type { AiTrainingPlanOutput, AiTrainingPlanSection } from '@server/dto/training'
import { mulberry32, randomItem } from '@server/service/domain/training/prng'
import {
    correctBaseBpm,
    correctBpmRatio,
    correctTrainingPlan,
    distributeMeasures,
    measureUsOf,
    trainingBoundsOf,
    trainingMaxNpsOf,
    trainingSeedOf,
    trainingShortIdOf,
    trainingSpecInputOf,
    trainingSubtitleOf,
    trainingTitleOf,
    warmupMaxMeasuresOf,
} from '@server/service/domain/training/spec'
import {
    GENERATOR_VERSION,
    TRAINING_BPM_RATIOS,
    TRAINING_LENGTH_US,
    TRAINING_TAIL_US,
    trainingKindsOfKey,
} from '@server/service/domain/training/training-constants'

const DENSITY = { limit_nps_ex: 12, max_seen_nps: 14 }
const BOUNDS = trainingBoundsOf(DENSITY, 'push')
const KEYS = ['tag:trill', 'tag:jack', 'tag:chord_3', 'tag:scratch_solo', 'warmup']
const ALLOWED = { kinds: [], lane_hints: [], keys: KEYS }
const ID = `tc_${'ab'.repeat(16)}`

const section = (partial: Partial<AiTrainingPlanSection> = {}): AiTrainingPlanSection => ({
    focus_key: 'tag:trill',
    kind: 'trill',
    weight: 2,
    lane_hint: 'all',
    chord_size: 1,
    scratch_ratio: 0,
    ln_ratio: 0,
    bpm_ratio: 1,
    intent: '',
    ...partial,
})

const plan = (sections: AiTrainingPlanSection[], baseBpm = 160, level = 0.5): AiTrainingPlanOutput => ({
    description: '',
    base_bpm: baseBpm,
    level,
    shape: 'flat',
    sections,
    advice: '',
})

describe('밀도 범위 (trainingBoundsOf)', () => {
    test('강도마다 한계 L 의 비로 본문 nps2 p90 범위를 정한다: build 0.9~1.0, push 1.0~1.1, over 1.15~1.3', () => {
        expect(trainingBoundsOf(DENSITY, 'build')).toMatchObject({ limit_nps: 12, p90_min: 10.8, p90_max: 12 })
        expect(trainingBoundsOf(DENSITY, 'push')).toMatchObject({ limit_nps: 12, p90_min: 12, p90_max: 13.2 })
        expect(trainingBoundsOf(DENSITY, 'over')).toMatchObject({ limit_nps: 12, p90_min: 13.8, p90_max: 15.6 })
        expect(BOUNDS).toMatchObject({ bpm_min: 100, bpm_max: 220, sections_min: 3, sections_max: 12, length_s: 60, warmup_max_s: 4.8 })
    })

    test('EX 한계가 없으면 본 최대 NPS, 그것도 없으면 기본 한계 12 를 쓰고 전역 [2, 60] 으로 자른다', () => {
        expect(trainingBoundsOf({ limit_nps_ex: null, max_seen_nps: 20 }, 'push')).toMatchObject({ p90_min: 20, p90_max: 22 })
        expect(trainingBoundsOf({ limit_nps_ex: null, max_seen_nps: null }, 'build')).toMatchObject({ p90_min: 10.8, p90_max: 12 })
        expect(trainingBoundsOf({ limit_nps_ex: 50, max_seen_nps: null }, 'over')).toMatchObject({ p90_min: 57.5, p90_max: 60 })
        expect(trainingBoundsOf({ limit_nps_ex: 1, max_seen_nps: null }, 'build')).toMatchObject({ p90_min: 2, p90_max: 2 })
    })
})

describe('BPM 보정', () => {
    test('기준 BPM 은 [100, 220] 안의 4 의 배수가 된다', () => {
        expect([97, 158, 161, 231, 150].map(correctBaseBpm)).toEqual([100, 160, 160, 220, 152])
    })

    test('배율은 가장 가까운 허용값이고, 유효 BPM 이 [60, 300] 밖이면 1 쪽으로 한 단계씩 옮긴다', () => {
        expect(correctBpmRatio(1.4, 160)).toBe(1.5)
        expect(correctBpmRatio(0.1, 100)).toBe(0.75)
        expect(correctBpmRatio(3, 220)).toBe(1.25)
        expect(correctBpmRatio(3, 148)).toBe(2)
    })
})

describe('구간 보정 (correctTrainingPlan)', () => {
    test('allowed.keys 밖 구간과 첫 구간이 아닌 워밍업은 버리고 원래 번호로 corrections 에 남긴다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan([
                section(),
                section({ focus_key: 'tag:stair', kind: 'stair' }),
                section({ focus_key: 'tag:jack', kind: 'jack' }),
                section({ focus_key: 'warmup', kind: 'stream' }),
                section(),
                section({ focus_key: 'cooldown', kind: 'rest' }),
            ]),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(spec.sections.map((item) => item.focus_key)).toEqual(['tag:trill', 'tag:jack', 'tag:trill'])
        expect(spec.sections.map((item) => item.index)).toEqual([0, 1, 2])
        expect(corrections).toContainEqual({ section: 1, field: 'focus_key', from: 'tag:stair', to: null })
        expect(corrections).toContainEqual({ section: 3, field: 'focus_key', from: 'warmup', to: null })
        expect(corrections).toContainEqual({ section: 5, field: 'focus_key', from: 'cooldown', to: null })
    })

    test('키의 종류 목록 밖 kind 는 목록의 첫 종류로 바꾸고 corrections 에 남긴다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan([section({ focus_key: 'warmup', kind: 'jack' }), section({ kind: 'stream' }), section({ focus_key: 'tag:jack', kind: 'burst' })]),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(spec.sections.map((item) => item.kind)).toEqual(['rest', 'trill', 'jack'])
        expect(corrections).toContainEqual({ section: 0, field: 'kind', from: 'jack', to: 'rest' })
        expect(corrections).toContainEqual({ section: 1, field: 'kind', from: 'stream', to: 'trill' })
        expect(corrections).toContainEqual({ section: 2, field: 'kind', from: 'burst', to: 'jack' })
    })

    test('남은 구간이 3개 미만이면 AI_OUTPUT_INVALID 로 거절한다', () => {
        const refused = (() => {
            try {
                correctTrainingPlan(plan([section(), section({ focus_key: 'tag:stair' }), section({ focus_key: 'lane:1' })]), {
                    allowed: ALLOWED,
                    bounds: BOUNDS,
                })
                return null
            } catch (error) {
                return error
            }
        })()
        expect(isAppError(refused) && refused.code).toBe('AI_OUTPUT_INVALID')
    })

    test('수치는 범위로 자르고 바뀐 값만 corrections 에 남기며 목표 밀도는 구간이 아니라 bounds 에 남는다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan(
                [
                    section({ focus_key: 'warmup', kind: 'rest', bpm_ratio: 0.8 }),
                    section({ focus_key: 'tag:chord_3', kind: 'chord_stream', chord_size: 6.4, weight: 30, scratch_ratio: 0.9, ln_ratio: -1 }),
                    section({ focus_key: 'tag:scratch_solo', kind: 'scratch_solo', chord_size: 3, scratch_ratio: 0.3 }),
                ],
                203,
                -0.2,
            ),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(spec.base_bpm).toBe(204)
        expect(spec.bounds).toEqual({ limit_nps: 12, p90_min: 12, p90_max: 13.2, level: 0, p90_target: 12, shape: 'flat' })
        expect(
            spec.sections.map(({ chord_size, scratch_ratio, ln_ratio, bpm }) => ({
                chord_size,
                scratch_ratio,
                ln_ratio,
                bpm,
            })),
        ).toEqual([
            { chord_size: 1, scratch_ratio: 0, ln_ratio: 0, bpm: 153 },
            { chord_size: 4, scratch_ratio: 0.5, ln_ratio: 0, bpm: 204 },
            { chord_size: 1, scratch_ratio: 0, ln_ratio: 0, bpm: 204 },
        ])
        expect(spec.sections.every((item) => !Object.hasOwn(item, 'target_nps'))).toBe(true)
        expect(corrections).toEqual(
            expect.arrayContaining([
                { section: null, field: 'base_bpm', from: 203, to: 204 },
                { section: null, field: 'level', from: -0.2, to: 0 },
                { section: 0, field: 'bpm_ratio', from: 0.8, to: 0.75 },
                { section: 1, field: 'chord_size', from: 6.4, to: 4 },
                { section: 1, field: 'weight', from: 30, to: 10 },
                { section: 1, field: 'scratch_ratio', from: 0.9, to: 0.5 },
                { section: 1, field: 'ln_ratio', from: -1, to: 0 },
                { section: 2, field: 'chord_size', from: 3, to: 1 },
                { section: 2, field: 'scratch_ratio', from: 0.3, to: 0 },
            ]),
        )
        expect(corrections.some((correction) => correction.field === 'target_nps')).toBe(false)
    })

    test('목표 p90 의 1.25배를 가장 촘촘한 격자로 낼 수 없으면 기준 BPM 을 4 씩 220 까지 올리고 corrections 에 남긴다', () => {
        const jacks = [0, 1, 2].map(() => section({ focus_key: 'tag:jack', kind: 'jack' }))
        const p2 = trainingBoundsOf({ limit_nps_ex: 16, max_seen_nps: null }, 'push')
        const raised = correctTrainingPlan(plan(jacks, 100), { allowed: ALLOWED, bounds: p2 })
        expect(raised.spec.bounds.p90_target).toBe(16.8)
        expect(raised.spec.base_bpm).toBe(160)
        expect(trainingMaxNpsOf({ kind: 'jack', chord_size: 1, scratch_ratio: 0, bpm: 160 })).toBeGreaterThanOrEqual(16.8 * 1.25)
        expect(trainingMaxNpsOf({ kind: 'jack', chord_size: 1, scratch_ratio: 0, bpm: 156 })).toBeLessThan(16.8 * 1.25)
        expect(raised.corrections).toContainEqual({ section: null, field: 'base_bpm', from: 100, to: 160 })

        const p3 = trainingBoundsOf({ limit_nps_ex: 24, max_seen_nps: null }, 'over')
        expect(correctTrainingPlan(plan(jacks, 120, 1), { allowed: ALLOWED, bounds: p3 }).spec.base_bpm).toBe(220)

        const chords = [0, 1, 2].map(() => section({ focus_key: 'tag:chord_3', kind: 'chord_stream', chord_size: 3 }))
        expect(correctTrainingPlan(plan(chords, 120, 1), { allowed: ALLOWED, bounds: p3 }).spec.base_bpm).toBe(120)
    })

    test('워밍업은 곡 길이의 8% 안의 마디 수로 줄고 남는 시간은 다른 구간이 받는다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan([section({ focus_key: 'warmup', kind: 'stream', weight: 10 }), section({ weight: 1 }), section({ weight: 1 })]),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(warmupMaxMeasuresOf(160)).toBe(3)
        expect(spec.sections[0].measure_count).toBe(3)
        expect(spec.sections[0].measure_count * measureUsOf(160)).toBeLessThanOrEqual(TRAINING_LENGTH_US * 0.08)
        const total = spec.sections.reduce((sum, item) => sum + item.measure_count * measureUsOf(item.bpm), 0)
        expect(total).toBeGreaterThan(TRAINING_LENGTH_US - measureUsOf(160) - TRAINING_TAIL_US - measureUsOf(160))
        expect(corrections).toContainEqual(expect.objectContaining({ section: 0, field: 'measures', to: 3 }))
    })

    test('구간은 마디 1 부터 이어지고 마디 수 보정은 반올림 값과 다를 때만 남는다', () => {
        const { spec, corrections } = correctTrainingPlan(plan([section(), section({ weight: 5 }), section({ weight: 1 })]), {
            allowed: ALLOWED,
            bounds: BOUNDS,
        })
        expect(spec.sections[0].measure_from).toBe(1)
        spec.sections
            .slice(1)
            .forEach((item, index) => expect(item.measure_from).toBe(spec.sections[index].measure_from + spec.sections[index].measure_count))
        for (const correction of corrections.filter((item) => item.field === 'measures')) expect(correction.from).not.toBe(correction.to)
    })
})

describe('마디 분배 불변식 (distributeMeasures)', () => {
    test('무게·BPM 무작위 1,000조합에서 (a) 모든 구간 1마디 이상 (b)(c) A − 가장 짧은 마디 < 합 ≤ A', () => {
        const random = mulberry32(20261104)
        const bases = Array.from({ length: (220 - 100) / 4 + 1 }, (_, index) => 100 + index * 4)
        for (let round = 0; round < 1000; round += 1) {
            const baseBpm = randomItem(random, bases)
            const count = 3 + Math.floor(random() * 10)
            const sections = Array.from({ length: count }, () => ({
                weight: 1 + random() * 9,
                bpm: baseBpm * correctBpmRatio(randomItem(random, TRAINING_BPM_RATIOS), baseBpm),
            }))
            const { measures } = distributeMeasures(sections, baseBpm)
            const available = TRAINING_LENGTH_US - measureUsOf(baseBpm) - TRAINING_TAIL_US
            const total = measures.reduce((sum, value, index) => sum + value * measureUsOf(sections[index].bpm), 0)
            const shortest = Math.min(...sections.map((item) => measureUsOf(item.bpm)))
            expect(measures.every((value) => Number.isInteger(value) && value >= 1)).toBe(true)
            expect(total).toBeLessThanOrEqual(available)
            expect(total).toBeGreaterThan(available - shortest)
            expect(measureUsOf(baseBpm) + total).toBeLessThanOrEqual(TRAINING_LENGTH_US - TRAINING_TAIL_US)
        }
    })

    test('반올림이 넘치면 가장 긴 구간(동률이면 뒤)부터 줄이고 남는 시간은 무거운 구간부터 채운다', () => {
        const { measures } = distributeMeasures(
            [
                { weight: 1, bpm: 160 },
                { weight: 2, bpm: 160 },
                { weight: 1, bpm: 160 },
            ],
            160,
        )
        expect(measures).toEqual([10, 18, 10])
        const even = (count: number, bpm: number) => Array.from({ length: count }, () => ({ weight: 1, bpm }))
        expect(distributeMeasures(even(3, 120), 120).measures).toEqual([10, 9, 9])
        expect(distributeMeasures(even(7, 160), 160).measures).toEqual([8, 5, 5, 5, 5, 5, 5])
    })
})

describe('마디 분배 상한', () => {
    test('max 가 있는 구간은 그 마디 수를 넘지 않고 반올림 값은 상한 전 값으로 남는다', () => {
        const { rounded, measures } = distributeMeasures(
            [
                { weight: 4, bpm: 160, max: 3 },
                { weight: 3, bpm: 160 },
                { weight: 3, bpm: 160 },
            ],
            160,
        )
        expect(rounded[0]).toBe(15)
        expect(measures[0]).toBe(3)
        expect(measures.reduce((sum, count) => sum + count, 0)).toBe(38)
    })
})

describe('시드·제목·키 종류', () => {
    const corrected = correctTrainingPlan(plan([section(), section(), section()]), { allowed: ALLOWED, bounds: BOUNDS })

    test('시드는 사용자·시각·보정 결과로 정해지고 같은 입력이면 같다', () => {
        const seed = trainingSeedOf({ userId: 'u1', createdAt: 1, plan: corrected.spec })
        expect(trainingSeedOf({ userId: 'u1', createdAt: 1, plan: corrected.spec })).toBe(seed)
        expect(trainingSeedOf({ userId: 'u1', createdAt: 2, plan: corrected.spec })).not.toBe(seed)
        expect(Number.isInteger(seed) && seed >= 0 && seed < 2 ** 32).toBe(true)
    })

    test('공개 제목은 날짜와 강도뿐이고 부제는 id 의 16진 앞 8자다', () => {
        expect(trainingTitleOf(Date.UTC(2026, 10, 4, 23, 59), 'push')).toBe('R-BMS Training · 2026-11-04 · PUSH')
        expect(trainingShortIdOf(ID)).toBe('abababab')
        expect(trainingSubtitleOf(ID)).toBe('[abababab]')
        const input = trainingSpecInputOf({
            id: ID,
            userId: 'u1',
            createdAt: 0,
            intensity: 'over',
            plan: corrected.spec,
            corrections: corrected.corrections,
        })
        expect(input).toMatchObject({ generator_version: GENERATOR_VERSION, title: 'R-BMS Training · 1970-01-01 · OVER', subtitle: '[abababab]' })
    })

    test('약점 키마다 훈련 종류가 정해지고 모르는 키는 빈 목록이다', () => {
        expect(trainingKindsOfKey('tag:jack')).toEqual(['jack', 'chord_jack'])
        expect(trainingKindsOfKey('timing:trill')).toEqual(['trill'])
        expect(trainingKindsOfKey('combo:trill+scratch_mixed')).toEqual(['trill', 'scratch_mixed'])
        expect(trainingKindsOfKey('combo:trill+nothing')).toEqual([])
        expect(trainingKindsOfKey('lane:S')).toEqual(['stream', 'trill', 'chord_stream'])
        expect(trainingKindsOfKey('density:limit')).toEqual(['stream', 'chord_stream'])
        expect(trainingKindsOfKey('warmup')).toEqual(['rest', 'stream'])
        expect(trainingKindsOfKey('tag:constructor')).toEqual([])
        expect(trainingKindsOfKey('unknown')).toEqual([])
    })
})
