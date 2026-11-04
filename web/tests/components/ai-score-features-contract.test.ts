import { describe, expect, test } from 'bun:test'
import {
    AI_GOAL_KINDS,
    AI_GOAL_LIMIT,
    AI_GREEN_DIRECTIONS,
    AI_OFFSET_DIRECTIONS,
    AI_PRACTICE_LIMIT,
    AI_RECOMMEND_KINDS,
} from '../../src/server/dto/ai'
import { PATTERN_TAGS } from '../../src/server/service/domain/analysis/pattern-tags'
import { BPM_BANDS, HAND_KEYS, SP7_LANE_NAMES } from '../../src/server/service/domain/analysis/analysis-constants'
import { COMBO_TAGS } from '../../src/server/service/domain/analysis/tag-combos'
import {
    DENSITY_KEY_PARTS,
    SOFLAN_KEY_PHASES,
    WEAKNESS_KEY,
    WEAKNESS_KINDS,
    parseWeaknessKey,
} from '../../src/server/service/domain/analysis/weakness-key'
import { PROFILE_SAMPLE_LEVELS } from '../../src/server/dto/ai-play'
import {
    AI_COMBO_TAGS,
    AI_DATA_NEEDS,
    AI_DENSITY_PARTS,
    AI_HAND_KEYS,
    AI_LANE_NAMES,
    AI_PROFILE_SAMPLE_LEVELS,
    AI_SOFLAN_PHASES,
    AI_WEAKNESS_KINDS,
    aiComboKeyOf,
    splitAiWeaknessKey,
    AI_GOAL_KINDS as SCREEN_GOAL_KINDS,
    AI_GREEN_DIRECTION_LABEL,
    AI_OFFSET_DIRECTION_LABEL,
    AI_PATTERN_TAG_LABEL,
    AI_RECOMMEND_KIND_LABEL,
    AI_SCORE_FEATURE_FORM_LIMIT,
} from '../../src/entities/ai.type'
import { MESSAGES } from '../../src/shared/i18n/messages'

describe('점수 기반 AI 기능 화면과 서버의 계약', () => {
    test('패턴 태그·목표 종류·후보 종류·방향 이름이 서버와 같다', () => {
        expect(Object.keys(AI_PATTERN_TAG_LABEL)).toEqual([...PATTERN_TAGS])
        expect([...SCREEN_GOAL_KINDS]).toEqual([...AI_GOAL_KINDS])
        expect(Object.keys(MESSAGES.ko.pages.ai.goalForm.goalKind)).toEqual([...AI_GOAL_KINDS])
        expect(Object.keys(MESSAGES.ko.pages.ai.patternTag)).toEqual([...PATTERN_TAGS])
        expect(Object.keys(AI_RECOMMEND_KIND_LABEL)).toEqual([...AI_RECOMMEND_KINDS])
        expect(Object.keys(AI_OFFSET_DIRECTION_LABEL)).toEqual([...AI_OFFSET_DIRECTIONS])
        expect(Object.keys(AI_GREEN_DIRECTION_LABEL)).toEqual([...AI_GREEN_DIRECTIONS])
    })

    test('폼 한도가 서버와 같고 서버가 보내는 표본 부족 이유마다 문구가 있다', () => {
        expect(AI_SCORE_FEATURE_FORM_LIMIT).toEqual({
            PRACTICE_NAME_PREFIX: AI_PRACTICE_LIMIT.NAME_PREFIX,
            PRACTICE_NAME_SUFFIX_MAX: AI_PRACTICE_LIMIT.NAME_SUFFIX_MAX,
            GOAL_ACTIVE_MAX: AI_GOAL_LIMIT.ACTIVE_MAX,
            GOAL_COUNT_MAX: AI_GOAL_LIMIT.COUNT_MAX,
            GOAL_EX_RATE_MAX: AI_GOAL_LIMIT.EX_RATE_MAX_PERCENT,
            GOAL_TARGET_RESIDUAL_MIN: AI_GOAL_LIMIT.TARGET_RESIDUAL_MIN,
            GOAL_TARGET_RESIDUAL_MAX: AI_GOAL_LIMIT.TARGET_RESIDUAL_MAX,
            GOAL_TARGET_NPS_MIN: AI_GOAL_LIMIT.TARGET_NPS_MIN,
            GOAL_TARGET_NPS_MAX: AI_GOAL_LIMIT.TARGET_NPS_MAX,
        })
        const serverNeeds = [
            'table_records',
            'candidates',
            'chart_patterns',
            'goal_reached',
            'remaining_charts',
            'table',
            'week_plays',
            'recent_plays',
            'traced_plays',
            'play_trace',
            'play_metrics',
            'comparable_play',
            'weakness_profile',
        ]
        expect<string[]>([...AI_DATA_NEEDS].toSorted()).toEqual(serverNeeds.toSorted())
        expect(
            Object.keys(MESSAGES.ko.pages.ai.dataNeed)
                .filter((need) => need !== 'fallback')
                .toSorted(),
        ).toEqual(serverNeeds.toSorted())
    })
})

describe('약점 프로필 화면과 서버의 계약', () => {
    test('약점 종류·조합 태그·손·레인·소프란·밀도·표본 등급 이름이 서버와 같다', () => {
        expect([...AI_WEAKNESS_KINDS]).toEqual([...WEAKNESS_KINDS])
        expect([...AI_COMBO_TAGS]).toEqual([...COMBO_TAGS])
        expect([...AI_HAND_KEYS]).toEqual([...HAND_KEYS])
        expect([...AI_LANE_NAMES]).toEqual([...SP7_LANE_NAMES])
        expect([...AI_SOFLAN_PHASES]).toEqual([...SOFLAN_KEY_PHASES])
        expect([...AI_DENSITY_PARTS]).toEqual([...DENSITY_KEY_PARTS])
        expect([...AI_PROFILE_SAMPLE_LEVELS]).toEqual([...PROFILE_SAMPLE_LEVELS])
        expect(Object.keys(MESSAGES.ko.pages.ai.weaknessKind.hand)).toEqual([...HAND_KEYS])
        expect(Object.keys(MESSAGES.ko.pages.ai.weaknessKind.soflan)).toEqual([...SOFLAN_KEY_PHASES])
        expect(Object.keys(MESSAGES.ko.pages.ai.weaknessKind.density)).toEqual([...DENSITY_KEY_PARTS])
        expect(Object.keys(MESSAGES.ko.pages.ai.profile.sampleLevel)).toEqual([...PROFILE_SAMPLE_LEVELS])
    })

    test('화면이 서버가 만든 약점 키를 같은 종류와 인자로 읽고, 조합 키를 서버와 같게 만든다', () => {
        const keys = [
            WEAKNESS_KEY.tag('trill'),
            WEAKNESS_KEY.solo('scratch_mixed'),
            WEAKNESS_KEY.combo('scratch_mixed', 'ln_release_then_hit'),
            WEAKNESS_KEY.lane('S'),
            WEAKNESS_KEY.hand('shared'),
            WEAKNESS_KEY.timing('jack'),
            ...BPM_BANDS.map((band) => WEAKNESS_KEY.bpm(band)),
            WEAKNESS_KEY.soflan('settled'),
            WEAKNESS_KEY.fatigue(),
            WEAKNESS_KEY.density('limit'),
            WEAKNESS_KEY.bpContext('stair'),
            WEAKNESS_KEY.gaugeContext('dense'),
        ]
        for (const key of keys) {
            const server = parseWeaknessKey(key)
            const screen = splitAiWeaknessKey(key)
            expect(screen?.kind).toBe(server?.kind)
            expect(screen?.parts).toEqual(server?.arg === null || server?.arg === undefined ? (server?.tags ?? []) : [server.arg])
        }
        expect(splitAiWeaknessKey('unknown:trill')).toBeNull()
        expect(aiComboKeyOf('chord_2', 'dense')).toBe(WEAKNESS_KEY.combo('chord_2', 'dense'))
    })
})

describe('판별 기반 AI 기능 화면과 서버의 계약', () => {
    test('게이지·옵션·결과·비교 이유 이름이 서버와 같고, 판 id 꼴이 서버 경로 검사와 같다', async () => {
        const server = await import('../../src/server/dto/ai-play')
        const screen = await import('../../src/entities/ai.type')
        expect(Object.keys(screen.AI_GAUGE_LABEL)).toEqual([...server.AI_GAUGE_NAMES])
        expect(Object.keys(screen.AI_OPTION_LABEL)).toEqual([...server.AI_OPTION_NAMES])
        expect(Object.keys(screen.AI_GAUGE_OUTCOME_LABEL)).toEqual([...server.AI_GAUGE_OUTCOMES])
        expect(Object.keys(screen.AI_RUN_END_LABEL)).toEqual([...server.AI_RUN_ENDS])
        expect(Object.keys(screen.AI_COMPARE_REASON_LABEL).toSorted()).toEqual([...server.AI_COMPARE_REASONS].toSorted())
        for (const id of ['sc_0123abcd', 'a-b_c', 'x'.repeat(36), 'bad id', 'x'.repeat(37), '../a']) {
            expect(screen.isAiScoreId(id)).toBe(server.aiScoreIdSchema.safeParse(id).success)
        }
    })
})
