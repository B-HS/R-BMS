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
} from '@server/service/domain/training/spec'
import {
    GENERATOR_VERSION,
    TRAINING_BPM_RATIOS,
    TRAINING_LENGTH_US,
    TRAINING_TAIL_US,
    trainingKindsOfKey,
} from '@server/service/domain/training/training-constants'

const DENSITY = { comfort_nps: 8, limit_nps_ex: 12, max_seen_nps: 14 }
const BOUNDS = trainingBoundsOf(DENSITY, 'push')
const KEYS = ['tag:trill', 'tag:jack', 'tag:chord_3', 'tag:scratch_solo', 'warmup', 'cooldown']
const ALLOWED = { kinds: [], lane_hints: [], keys: KEYS }
const ID = `tc_${'ab'.repeat(16)}`

const section = (partial: Partial<AiTrainingPlanSection> = {}): AiTrainingPlanSection => ({
    focus_key: 'tag:trill',
    kind: 'trill',
    weight: 2,
    target_nps: 8,
    lane_hint: 'all',
    chord_size: 1,
    scratch_ratio: 0,
    ln_ratio: 0,
    bpm_ratio: 1,
    intent: '',
    ...partial,
})

const plan = (sections: AiTrainingPlanSection[], baseBpm = 160): AiTrainingPlanOutput => ({
    description: '',
    base_bpm: baseBpm,
    sections,
    advice: '',
})

describe('NPS 범위 (trainingBoundsOf)', () => {
    test('강도마다 편안·한계 밀도의 비로 범위를 정한다', () => {
        expect(trainingBoundsOf(DENSITY, 'build')).toMatchObject({ nps_min: 4.8, nps_max: 8, comfort_nps: 8 })
        expect(trainingBoundsOf(DENSITY, 'push')).toMatchObject({ nps_min: 6.4, nps_max: 12 })
        expect(trainingBoundsOf(DENSITY, 'over')).toMatchObject({ nps_min: 8, nps_max: 13.8 })
        expect(BOUNDS).toMatchObject({ bpm_min: 100, bpm_max: 220, sections_min: 3, sections_max: 12, length_s: 60 })
    })

    test('밀도가 없으면 본 최대 NPS, 그것도 없으면 기본 한계 12 와 그 0.7 을 쓰고 전역 [2, 40] 으로 자른다', () => {
        expect(trainingBoundsOf({ comfort_nps: null, limit_nps_ex: null, max_seen_nps: 20 }, 'push')).toMatchObject({ nps_min: 11.2, nps_max: 20 })
        expect(trainingBoundsOf({ comfort_nps: null, limit_nps_ex: null, max_seen_nps: null }, 'build')).toMatchObject({
            nps_min: 5.04,
            nps_max: 8.4,
        })
        expect(trainingBoundsOf({ comfort_nps: 1, limit_nps_ex: 50, max_seen_nps: null }, 'over')).toMatchObject({ nps_min: 2, nps_max: 40 })
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
    test('allowed.keys 밖 구간은 버리고 원래 번호로 corrections 에 남긴다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan([section(), section({ focus_key: 'tag:stair', kind: 'stair' }), section({ focus_key: 'tag:jack', kind: 'jack' }), section()]),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(spec.sections.map((item) => item.focus_key)).toEqual(['tag:trill', 'tag:jack', 'tag:trill'])
        expect(spec.sections.map((item) => item.index)).toEqual([0, 1, 2])
        expect(corrections).toContainEqual({ section: 1, field: 'focus_key', from: 'tag:stair', to: null })
    })

    test('키의 종류 목록 밖 kind 는 목록의 첫 종류로 바꾸고 corrections 에 남긴다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan([section({ kind: 'stream' }), section({ focus_key: 'tag:jack', kind: 'burst' }), section({ focus_key: 'warmup', kind: 'jack' })]),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(spec.sections.map((item) => item.kind)).toEqual(['trill', 'jack', 'rest'])
        expect(corrections).toContainEqual({ section: 0, field: 'kind', from: 'stream', to: 'trill' })
        expect(corrections).toContainEqual({ section: 1, field: 'kind', from: 'burst', to: 'jack' })
        expect(corrections).toContainEqual({ section: 2, field: 'kind', from: 'jack', to: 'rest' })
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

    test('수치는 범위로 자르고 바뀐 값만 corrections 에 남긴다', () => {
        const { spec, corrections } = correctTrainingPlan(
            plan(
                [
                    section({
                        focus_key: 'tag:chord_3',
                        kind: 'chord_stream',
                        chord_size: 6.4,
                        weight: 30,
                        target_nps: 50,
                        scratch_ratio: 0.9,
                        ln_ratio: -1,
                    }),
                    section({ focus_key: 'tag:scratch_solo', kind: 'scratch_solo', chord_size: 3, scratch_ratio: 0.3, target_nps: 1 }),
                    section({ focus_key: 'warmup', kind: 'rest', target_nps: 9, bpm_ratio: 0.8 }),
                ],
                203,
            ),
            { allowed: ALLOWED, bounds: BOUNDS },
        )
        expect(spec.base_bpm).toBe(204)
        expect(
            spec.sections.map(({ chord_size, scratch_ratio, ln_ratio, target_nps, bpm }) => ({
                chord_size,
                scratch_ratio,
                ln_ratio,
                target_nps,
                bpm,
            })),
        ).toEqual([
            { chord_size: 4, scratch_ratio: 0.5, ln_ratio: 0, target_nps: 12, bpm: 204 },
            { chord_size: 1, scratch_ratio: 0, ln_ratio: 0, target_nps: 6.4, bpm: 204 },
            { chord_size: 1, scratch_ratio: 0, ln_ratio: 0, target_nps: 2.55, bpm: 153 },
        ])
        expect(corrections).toEqual(
            expect.arrayContaining([
                { section: null, field: 'base_bpm', from: 203, to: 204 },
                { section: 0, field: 'chord_size', from: 6.4, to: 4 },
                { section: 0, field: 'weight', from: 30, to: 10 },
                { section: 0, field: 'target_nps', from: 50, to: 12 },
                { section: 0, field: 'scratch_ratio', from: 0.9, to: 0.5 },
                { section: 0, field: 'ln_ratio', from: -1, to: 0 },
                { section: 1, field: 'chord_size', from: 3, to: 1 },
                { section: 1, field: 'scratch_ratio', from: 0.3, to: 0 },
                { section: 2, field: 'bpm_ratio', from: 0.8, to: 0.75 },
                { section: 2, field: 'target_nps', from: 9, to: 2.55 },
            ]),
        )
        expect(corrections.some((correction) => correction.section === 0 && correction.field === 'kind')).toBe(false)
    })

    test('목표 NPS 는 가장 촘촘한 격자로 낼 수 있는 값을 넘지 않는다', () => {
        const fast = section({ focus_key: 'tag:jack', kind: 'jack', target_nps: 40 })
        const { spec } = correctTrainingPlan(plan([fast, fast, fast], 100), { allowed: ALLOWED, bounds: trainingBoundsOf(DENSITY, 'over') })
        const ceiling = trainingMaxNpsOf({ kind: 'jack', chord_size: 1, scratch_ratio: 0, bpm: 100 })
        expect(ceiling).toBeCloseTo((32 * 100) / 240)
        expect(spec.sections[0].target_nps).toBe(Math.min(trainingBoundsOf(DENSITY, 'over').nps_max, ceiling))
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
