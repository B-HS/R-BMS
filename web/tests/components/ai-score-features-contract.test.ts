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
    AI_DATA_NEEDS,
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
        ]
        expect<string[]>([...AI_DATA_NEEDS].toSorted()).toEqual(serverNeeds.toSorted())
        expect(
            Object.keys(MESSAGES.ko.pages.ai.dataNeed)
                .filter((need) => need !== 'fallback')
                .toSorted(),
        ).toEqual(serverNeeds.toSorted())
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
