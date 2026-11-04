import { describe, expect, test } from 'bun:test'
import {
    AI_TRAINING_AUTO_FOCUS,
    AI_TRAINING_CORRECTION_FIELDS,
    AI_TRAINING_EDGE_KEYS,
    AI_TRAINING_ERROR_CODE,
    AI_TRAINING_INTENSITIES,
    AI_TRAINING_LANE_HINTS,
    AI_TRAINING_PATTERN_KINDS,
    AI_TRAINING_PLAYS_SHOWN,
    AI_HUB_TABS,
    aiHubHref,
    aiTrainingHref,
    canAiTrainingPickFocus,
    formatAiWeaknessDelta,
    isAiTrainingChartId,
    isAiTrainingPositionalKey,
    trainableAiWeaknessesOf,
} from '../../src/entities/ai.type'
import { LOCALES } from '../../src/shared/i18n/locale'
import { MESSAGES } from '../../src/shared/i18n/messages'
import {
    TRAINING_AUTO_FOCUS,
    TRAINING_INTENSITIES,
    TRAINING_LANE_HINTS,
    TRAINING_PATTERN_KINDS,
    trainingChartIdSchema,
} from '../../src/server/dto/training'
import { ERROR_CODE } from '../../src/server/lib/error-code'
import { focusWeaknessOf, trainableWeaknessesOf } from '../../src/server/service/domain/ai/training-plan.service'
import { TRAINING_CORRECTION_FIELDS } from '../../src/server/service/domain/training/spec'
import { TRAINING_EDGE_KEYS } from '../../src/server/service/domain/training/training-constants'
import { TRAINING_PLAYS_SHOWN } from '../../src/server/service/domain/training/training.service'
import { trainingProfile, trainingWeakness } from './ai-training-fixture'

const SERVER_ID = `tc_${'a1'.repeat(16)}`

describe('훈련곡 화면이 서버 계약을 거울로 갖는 값', () => {
    test('강도·자동 초점·패턴 종류·레인 힌트·양끝 구간 키·보정 항목·표시 판 수가 서버와 같다', () => {
        expect(AI_TRAINING_INTENSITIES).toEqual(TRAINING_INTENSITIES)
        expect(AI_TRAINING_AUTO_FOCUS).toBe(TRAINING_AUTO_FOCUS)
        expect(AI_TRAINING_PATTERN_KINDS).toEqual(TRAINING_PATTERN_KINDS)
        expect(AI_TRAINING_LANE_HINTS).toEqual(TRAINING_LANE_HINTS)
        expect(AI_TRAINING_EDGE_KEYS).toEqual(TRAINING_EDGE_KEYS)
        expect(AI_TRAINING_CORRECTION_FIELDS).toEqual(TRAINING_CORRECTION_FIELDS)
        expect(AI_TRAINING_PLAYS_SHOWN).toBe(TRAINING_PLAYS_SHOWN)
    })

    test('화면이 반응하는 오류 코드가 서버 코드와 같다', () => {
        expect(AI_TRAINING_ERROR_CODE).toEqual({
            LIMIT: ERROR_CODE.TRAINING_CHART_LIMIT,
            FOLDER_UNAVAILABLE: ERROR_CODE.TRAINING_FOLDER_UNAVAILABLE,
            NOT_FOUND: ERROR_CODE.TRAINING_CHART_NOT_FOUND,
        })
    })

    test('훈련곡 id 형식 검사가 서버 스키마와 같은 id 를 받고 같은 id 를 거절한다', () => {
        for (const id of [SERVER_ID, `tc_${'0'.repeat(32)}`, 'tc_', `tc_${'A1'.repeat(16)}`, `tc_${'a1'.repeat(17)}`, `uf_${'a1'.repeat(16)}`, '']) {
            expect(isAiTrainingChartId(id), id).toBe(trainingChartIdSchema.safeParse(id).success)
        }
    })

    test('훈련곡 탭은 AI 허브 탭에 있고 주소는 약점 키를 인코딩해 싣는다', () => {
        expect(AI_HUB_TABS).toContain('training')
        expect(aiHubHref('training', 'combo:trill+stair')).toBe('/ai?tab=training&focus=combo%3Atrill%2Bstair')
        expect(aiTrainingHref(SERVER_ID)).toBe(`/ai/training/${SERVER_ID}`)
    })
})

describe('훈련곡 화면의 약점 고르기 규칙', () => {
    const profile = trainingProfile()

    test('고를 수 있는 약점과 자동 가능 여부가 서버의 훈련곡 만들기 검사와 같다', () => {
        expect(trainableAiWeaknessesOf(profile.weaknesses).map((item) => item.key)).toEqual(
            profile.weaknesses.filter((item) => focusWeaknessOf(profile, item.key) !== null).map((item) => item.key),
        )
        expect(canAiTrainingPickFocus(profile.weaknesses)).toBe(trainableWeaknessesOf(profile).length > 0)
    })

    test('표본이 너무 작은 약점만 있으면 자동은 안 되지만 그 약점은 고를 수 있고 서버도 같다', () => {
        const small = trainingProfile({
            weaknesses: [trainingWeakness({ key: 'tag:jack', kind: 'tag', sample_level: 'insufficient', training_kinds: ['jack'] })],
        })
        expect(canAiTrainingPickFocus(small.weaknesses)).toBe(false)
        expect(trainableWeaknessesOf(small)).toEqual([])
        expect(trainableAiWeaknessesOf(small.weaknesses)).toHaveLength(1)
        expect(focusWeaknessOf(small, 'tag:jack')).not.toBeNull()
    })

    test('레인과 손 초점만 RANDOM 계열 옵션 안내가 필요하다', () => {
        expect(['lane:1', 'lane:S', 'hand:left', 'hand:scratch'].map(isAiTrainingPositionalKey)).toEqual([true, true, true, true])
        expect(
            ['tag:trill', 'combo:trill+stair', 'timing:jack', 'density:limit', 'fatigue', 'bpm:150-170', 'nothing'].map(isAiTrainingPositionalKey),
        ).toEqual([false, false, false, false, false, false, false])
    })

    test('전후 차이는 부호를 붙여 쓰고 밀도는 NPS 로 쓴다', () => {
        expect(formatAiWeaknessDelta('tag:trill', 'ex_residual', 0.04)).toBe('+4.0%p')
        expect(formatAiWeaknessDelta('tag:trill', 'ex_residual', -0.025)).toBe('-2.5%p')
        expect(formatAiWeaknessDelta('timing:jack', 'ms', -1.25)).toBe('-1.3 ms')
        expect(formatAiWeaknessDelta('density:limit', 'nps', 1.5)).toBe('+1.5 NPS')
        expect(formatAiWeaknessDelta('density:limit', 'nps', null)).toBe('—')
    })
})

describe('훈련곡 화면 문구 카탈로그', () => {
    test('패턴 종류·레인 힌트·양끝 구간·보정 항목·강도마다 세 언어 문구가 있다', () => {
        for (const locale of LOCALES) {
            const training = MESSAGES[locale].pages.ai.training
            for (const kind of AI_TRAINING_PATTERN_KINDS) expect(training.kind[kind], `${locale}:kind.${kind}`).toBeTruthy()
            for (const hint of AI_TRAINING_LANE_HINTS) expect(training.laneHint[hint], `${locale}:laneHint.${hint}`).toBeTruthy()
            for (const edge of AI_TRAINING_EDGE_KEYS) expect(training.edge[edge], `${locale}:edge.${edge}`).toBeTruthy()
            for (const field of AI_TRAINING_CORRECTION_FIELDS) expect(training.corrections.field[field], `${locale}:field.${field}`).toBeTruthy()
            for (const intensity of AI_TRAINING_INTENSITIES) {
                expect(training.form.intensity[intensity].name, `${locale}:intensity.${intensity}`).toBeTruthy()
                expect(training.form.intensity[intensity].description, `${locale}:intensity.${intensity}`).toBeTruthy()
            }
            expect(training.optionNotice, `${locale}:optionNotice`).toBeTruthy()
            expect(training.limitRaced, `${locale}:limitRaced`).toBeTruthy()
        }
    })
})
