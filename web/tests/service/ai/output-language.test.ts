import { describe, expect, test } from 'bun:test'
import { AI_OUTPUT_LANGUAGE_NAME } from '@server/service/domain/ai/ai-digest'
import { hashAiRunInput } from '@server/service/domain/ai/feature-runner.service'
import { aiGaugeAdviceSpecOf, buildAiGaugeAdviceSystemPrompt } from '@server/service/domain/ai/gauge-advice.service'
import { aiGoalSpecOf, buildAiGoalSystemPrompt } from '@server/service/domain/ai/goal-plan.service'
import { JUDGE_OFFSET_EFFECT_PHRASE, aiJudgeHabitSpecOf, buildAiJudgeHabitSystemPrompt } from '@server/service/domain/ai/judge-habit.service'
import { aiPlayCompareSpecOf, buildAiPlayCompareSystemPrompt } from '@server/service/domain/ai/play-compare.service'
import { aiPlayReportSpecOf, buildAiPlayReportSystemPrompt } from '@server/service/domain/ai/play-report.service'
import {
    AI_PRACTICE_DEFAULT_SUFFIX,
    aiPracticeSpecOf,
    buildAiPracticeSystemPrompt,
    refinePracticeOutput,
} from '@server/service/domain/ai/practice-folder.service'
import { aiRecommendSpecOf, buildAiRecommendSystemPrompt } from '@server/service/domain/ai/recommend.service'
import { buildAiRecordQuerySystemPrompt } from '@server/service/domain/ai/record-query.service'
import { aiReplayCoachSpecOf, buildAiReplayCoachSystemPrompt } from '@server/service/domain/ai/replay-coach.service'
import { aiWeeklySpecOf, buildAiWeeklySystemPrompt } from '@server/service/domain/ai/weekly-summary.service'
import { LOCALES } from '@shared/i18n/locale'
import { md5Of, registerScoreKey, setupScoreFeatures, SCORE_USER, levelRecords, makeTable, recordOf } from '../../support/ai-score-fixture'

const PROMPT_BUILDERS = {
    gauge_advice: buildAiGaugeAdviceSystemPrompt,
    goal_plan: buildAiGoalSystemPrompt,
    judge_habit: buildAiJudgeHabitSystemPrompt,
    play_compare: buildAiPlayCompareSystemPrompt,
    play_report: buildAiPlayReportSystemPrompt,
    practice_folder: buildAiPracticeSystemPrompt,
    recommend: buildAiRecommendSystemPrompt,
    record_query: buildAiRecordQuerySystemPrompt,
    replay_coach: buildAiReplayCoachSystemPrompt,
    weekly_summary: buildAiWeeklySystemPrompt,
}

const SPECS = [
    aiGaugeAdviceSpecOf,
    aiGoalSpecOf,
    aiJudgeHabitSpecOf,
    aiPlayCompareSpecOf,
    aiPlayReportSpecOf,
    aiPracticeSpecOf,
    aiRecommendSpecOf,
    aiReplayCoachSpecOf,
    aiWeeklySpecOf,
]

const TARGET = { provider: 'anthropic', model: 'claude-test' } as const
const DIGEST = { key: 'digest' }
const table = makeTable()
const RECORDS = { ...levelRecords(table, { '1': 'Hard', '2': 'Easy' }), [md5Of(300)]: recordOf('Failed', 85) }

describe('AI 결과 언어 = 화면 언어', () => {
    test('모든 기능 프롬프트가 그 언어로 쓰라고 하고 다른 언어 이름을 섞지 않는다', () => {
        for (const [feature, build] of Object.entries(PROMPT_BUILDERS)) {
            for (const language of LOCALES) {
                const prompt = build(language)
                expect(prompt, `${feature}:${language}`).toContain(AI_OUTPUT_LANGUAGE_NAME[language])
                for (const other of LOCALES.filter((locale) => locale !== language))
                    expect(prompt, `${feature}:${language}`).not.toContain(AI_OUTPUT_LANGUAGE_NAME[other])
            }
        }
    })

    test('같은 입력도 언어마다 캐시 키가 다르다', () => {
        for (const specOf of SPECS) {
            const hashes = LOCALES.map((language) => hashAiRunInput({ ...specOf(language), digest: DIGEST }, TARGET))
            expect(new Set(hashes).size, specOf('en').feature).toBe(LOCALES.length)
        }
    })

    test('JUDGE OFFSET 설명 문구와 연습 폴더 기본 이름도 그 언어로 쓴다', () => {
        for (const language of LOCALES) {
            expect(buildAiJudgeHabitSystemPrompt(language)).toContain(JUDGE_OFFSET_EFFECT_PHRASE[language])
            const refined = refinePracticeOutput(
                { name_suffix: '', description: '', items: [{ md5: md5Of(1), memo: '' }] },
                { candidates: [{ md5: md5Of(1) }], source: 'goal', language },
            )
            expect(refined?.name_suffix).toBe(AI_PRACTICE_DEFAULT_SUFFIX[language].goal)
        }
    })

    test('다른 언어로 만든 해석은 현재 결과가 아니고, 언어를 바꿔 다시 만들면 각 언어 캐시를 쓴다', async () => {
        const context = setupScoreFeatures({
            records: RECORDS,
            output: { summary: 'Into level 3.', picks: [{ md5: md5Of(300), reason: 'Nearly cleared.', focus_key: `chart.${md5Of(300)}` }] },
        })
        await registerScoreKey(context)

        expect(await context.recommend.interpret({ language: 'en', userId: SCORE_USER })).toMatchObject({ ok: true, value: { cached: false } })
        expect((await context.recommend.view({ language: 'en', userId: SCORE_USER }))?.ai).toMatchObject({ is_current: true })
        expect((await context.recommend.view({ language: 'ko', userId: SCORE_USER }))?.ai).toMatchObject({ is_current: false })

        expect(await context.recommend.interpret({ language: 'ko', userId: SCORE_USER })).toMatchObject({ ok: true, value: { cached: false } })
        expect(await context.recommend.interpret({ language: 'en', userId: SCORE_USER })).toMatchObject({ ok: true, value: { cached: true } })
        expect(context.anthropic.generateCalls.map((call) => call.system)).toEqual([
            buildAiRecommendSystemPrompt('en'),
            buildAiRecommendSystemPrompt('ko'),
        ])
    })
})
