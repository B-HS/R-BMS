import { describe, expect, test } from 'bun:test'
import { aiTrainingPlanOutputSchema, trainingChartCreateSchema } from '@server/dto/training'
import {
    TRAINING_DIGEST_LIMIT,
    aiTrainingPlanSpecOf,
    refineTrainingPlanOutput,
    trainingPlanDigestOf,
    trainingPlanSubjectKey,
} from '@server/service/domain/ai/training-plan.service'
import { WEAKNESS_KEY_MAX_LENGTH } from '@server/service/domain/analysis/weakness-key'
import { trainingBoundsOf } from '@server/service/domain/training/spec'
import { planFromDigest, trainingProfileOf, trainingWeaknessOf } from '../../support/training-fixture'

const AI_RESULT_SUBJECT_KEY_MAX = 128

const manyWeaknesses = () => [
    ...['tag:trill', 'tag:jack', 'tag:stair', 'tag:stream', 'tag:scratch_solo', 'tag:chord_2', 'tag:chord_3', 'tag:dense', 'tag:soflan'].map((key) =>
        trainingWeaknessOf(key, 'tag'),
    ),
    trainingWeaknessOf('combo:jack+trill', 'combo'),
    trainingWeaknessOf('timing:trill', 'timing', { training_kinds: [] }),
    trainingWeaknessOf('tag:ln_head', 'tag', { sample_level: 'insufficient' }),
]

describe('trainingPlanDigestOf', () => {
    test('auto 는 훈련할 수 있고 표본이 low 이상인 약점 상위 8개와 warmup·cooldown 을 허용한다', () => {
        const profile = trainingProfileOf({ weaknesses: manyWeaknesses() })
        const digest = trainingPlanDigestOf(profile, [], { focus_key: 'auto', intensity: 'push' }, { frontierLevel: '★10' })
        expect(digest.weaknesses).toHaveLength(TRAINING_DIGEST_LIMIT.WEAKNESSES)
        expect(digest.weaknesses.map((item) => item.key)).not.toContain('timing:trill')
        expect(digest.weaknesses.map((item) => item.key)).not.toContain('tag:ln_head')
        expect(digest.allowed.keys).toEqual([...digest.weaknesses.map((item) => item.key), 'warmup', 'cooldown'])
        expect(digest.allowed.kinds).toContain('rest')
        expect(digest.allowed.kinds).toContain('trill')
        expect(digest.bounds).toEqual(trainingBoundsOf(profile.density, 'push'))
        expect(digest.frontier_level).toBe('★10')
        expect(digest.request).toEqual({ focus_key: 'auto', intensity: 'push' })
    })

    test('키를 고르면 그 키와 태그를 나누는 조합만 남고 허용 키는 그 키와 warmup·cooldown 이다', () => {
        const profile = trainingProfileOf({ weaknesses: manyWeaknesses() })
        const digest = trainingPlanDigestOf(profile, [], { focus_key: 'tag:trill', intensity: 'build' }, { frontierLevel: null })
        expect(digest.weaknesses.map((item) => item.key)).toEqual(['tag:trill', 'combo:jack+trill'])
        expect(digest.allowed.keys).toEqual(['tag:trill', 'warmup', 'cooldown'])
    })

    test('previous 는 최근 3곡까지만 싣는다', () => {
        const history = Array.from({ length: 5 }, (_, index) => ({
            created_at: index,
            focus_keys: ['tag:trill'],
            kinds: ['trill'],
            result_delta: null,
        }))
        const digest = trainingPlanDigestOf(trainingProfileOf(), history, { focus_key: 'auto', intensity: 'over' }, { frontierLevel: null })
        expect(digest.previous.map((item) => item.created_at)).toEqual([0, 1, 2])
    })
})

describe('refineTrainingPlanOutput', () => {
    const digest = trainingPlanDigestOf(trainingProfileOf(), [], { focus_key: 'auto', intensity: 'push' }, { frontierLevel: null })

    test('허용 키 구간이 3개 미만이면 거절하고, 아니면 구간을 그대로 두고 글만 한 줄로 다듬는다', () => {
        const plan = planFromDigest(digest)
        const tooFew = {
            ...plan,
            sections: plan.sections.map((section, index) => (index < plan.sections.length - 2 ? { ...section, focus_key: 'lane:1' } : section)),
        }
        expect(refineTrainingPlanOutput(tooFew, digest)).toBeNull()

        const messy = aiTrainingPlanOutputSchema.parse({
            ...plan,
            description: '첫 줄\n둘째 줄',
            advice: '조언\u0007 끝',
            sections: plan.sections.map((section) => ({ ...section, intent: 'a\tb' })),
        })
        const refined = refineTrainingPlanOutput(messy, digest)
        expect(refined?.description).toBe('첫 줄둘째 줄')
        expect(refined?.advice).toBe('조언 끝')
        expect(refined?.sections.map((section) => section.intent)).toEqual(plan.sections.map(() => 'ab'))
        expect(refined?.sections.map((section) => section.focus_key)).toEqual(plan.sections.map((section) => section.focus_key))
        expect(aiTrainingPlanOutputSchema.safeParse(refined).success).toBe(true)
    })
})

describe('기능 사양과 캐시 키', () => {
    test('캐시 키는 train:<focus_key>:<intensity> 이고 가장 긴 키도 ai_result.subject_key 에 들어간다', () => {
        expect(trainingPlanSubjectKey({ focus_key: 'tag:trill', intensity: 'push' })).toBe('train:tag:trill:push')
        expect(trainingPlanSubjectKey({ focus_key: 'x'.repeat(WEAKNESS_KEY_MAX_LENGTH), intensity: 'build' }).length).toBeLessThanOrEqual(
            AI_RESULT_SUBJECT_KEY_MAX,
        )
    })

    test('프롬프트는 화면 언어를 쓰고 기능 이름·버전이 고정이다', () => {
        const spec = aiTrainingPlanSpecOf('ja')
        expect(spec).toMatchObject({ feature: 'training_plan', promptVersion: 1, maxOutputTokens: 8000, effort: 'low', schemaName: 'training_plan' })
        expect(spec.system).toContain('Japanese')
        expect(spec.system).toContain('allowed.keys')
    })
})

describe('trainingChartCreateSchema', () => {
    test('auto 와 문법에 맞는 약점 키만 받고 다른 필드·64자 초과·모르는 접두사·태그는 거절한다', () => {
        expect(trainingChartCreateSchema.safeParse({ focus_key: 'auto', intensity: 'push' }).success).toBe(true)
        expect(trainingChartCreateSchema.safeParse({ focus_key: 'combo:jack+trill', intensity: 'over' }).success).toBe(true)
        expect(trainingChartCreateSchema.safeParse({ focus_key: `tag:${'a'.repeat(61)}`, intensity: 'push' }).success).toBe(false)
        expect(trainingChartCreateSchema.safeParse({ focus_key: 'weird:trill', intensity: 'push' }).success).toBe(false)
        expect(trainingChartCreateSchema.safeParse({ focus_key: 'tag:nope', intensity: 'push' }).success).toBe(false)
        expect(trainingChartCreateSchema.safeParse({ focus_key: 'auto', intensity: 'max' }).success).toBe(false)
        expect(trainingChartCreateSchema.safeParse({ focus_key: 'auto', intensity: 'push', title: 'x' }).success).toBe(false)
    })
})
