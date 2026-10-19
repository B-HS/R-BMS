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
import {
    AI_DATA_NEED_LABEL,
    AI_GOAL_KIND_LABEL,
    AI_GREEN_DIRECTION_LABEL,
    AI_OFFSET_DIRECTION_LABEL,
    AI_PATTERN_TAG_LABEL,
    AI_RECOMMEND_KIND_LABEL,
    AI_SCORE_FEATURE_FORM_LIMIT,
} from '../../src/entities/ai.type'

describe('점수 기반 AI 기능 화면과 서버의 계약', () => {
    test('패턴 태그·목표 종류·후보 종류·방향 이름이 서버와 같다', () => {
        expect(Object.keys(AI_PATTERN_TAG_LABEL)).toEqual([...PATTERN_TAGS])
        expect(Object.keys(AI_GOAL_KIND_LABEL)).toEqual([...AI_GOAL_KINDS])
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
        ]
        expect(Object.keys(AI_DATA_NEED_LABEL).toSorted()).toEqual(serverNeeds.toSorted())
    })
})
