import { describe, expect, test } from 'bun:test'
import { createChartTagLoader } from '@server/compose/ai-score-features'
import type { WeaknessProfileCell, WeaknessProfileView } from '@server/dto/ai-play'
import { FOLDER_LIMIT } from '@server/dto/folder'
import { ASSIST_GRADE_RULE, DIGEST_IS_DATA_RULE } from '@server/service/domain/ai/ai-digest'
import { buildAiGoalSystemPrompt, computeGoalProgress, createAiGoalService, refineGoalPlanOutput } from '@server/service/domain/ai/goal-plan.service'
import { buildAiPracticeSystemPrompt, createAiPracticeFolderService, refinePracticeOutput } from '@server/service/domain/ai/practice-folder.service'
import {
    AI_RECOMMEND_FEATURE,
    buildAiRecommendSystemPrompt,
    createAiRecommendService,
    refineRecommendOutput,
    type TableFeatureDeps,
} from '@server/service/domain/ai/recommend.service'
import { readTableProgress } from '@server/service/domain/ai/table-progress'
import { computeChartFeatures, type ChartFeatures } from '@server/service/domain/analysis/chart-features'
import { FAKE_USAGE } from '../../support/ai-memory-db'
import { SCORE_NOW, SCORE_USER, levelRecords, makeTable, md5Of, recordOf, registerScoreKey, setupScoreFeatures } from '../../support/ai-score-fixture'
import { makePatternDocument } from '../../support/chart-pattern-fixture'
import { trainingProfileOf, trainingWeaknessOf } from '../../support/training-fixture'

const table = makeTable()
const RECORDS = { ...levelRecords(table, { '1': 'Hard', '2': 'Easy' }), [md5Of(300)]: recordOf('Failed', 85) }

describe('다음 도전 곡 추천 (기능 5)', () => {
    test('보기는 결정론 후보와 경계를 주고 공급자를 부르지 않는다', async () => {
        const context = setupScoreFeatures({ records: RECORDS, tags: new Map([[md5Of(300), { trill: 0.3, jack: 0.02 }]]) })
        await registerScoreKey(context)
        const view = await context.recommend.view({ language: 'ko', userId: SCORE_USER })
        expect(view?.frontier?.level).toBe('3')
        expect(view?.candidates[0]).toMatchObject({
            md5: md5Of(300),
            kind: 'failed',
            lamp: 'Failed',
            ex_rate: 85,
            tags: [{ tag: 'trill', ratio: 0.3 }],
        })
        expect(view?.pattern_coverage).toEqual({ with_features: 1, candidates: view?.candidates.length ?? 0, stale_features: 0 })
        expect(view?.notice).toBeNull()
        expect(view?.ai).toBeNull()
        expect(context.anthropic.generateCalls).toHaveLength(0)
        expect(await context.recommend.view({ language: 'ko', userId: SCORE_USER, tableId: 'missing' })).toBeNull()
    })

    test('옛 분석 버전으로 쓴 곡 수를 pattern_coverage.stale_features 로 알린다', async () => {
        const tags = new Map<string, Record<string, number>>([
            [md5Of(300), { trill: 0.3 }],
            [md5Of(301), { jack: 0.2 }],
        ])
        const context = setupScoreFeatures({ records: RECORDS, tags, staleChartMd5s: [md5Of(301), md5Of(999)] })
        const view = await context.recommend.view({ language: 'ko', userId: SCORE_USER })
        expect(view?.pattern_coverage).toMatchObject({ with_features: 2, stale_features: 1 })
    })

    test('표 기록이 하나도 없으면 AI 를 부르지 않고 표본 부족을 돌려준다', async () => {
        const context = setupScoreFeatures()
        await registerScoreKey(context)
        expect(await context.recommend.interpret({ language: 'ko', userId: SCORE_USER })).toEqual({
            ok: false,
            reason: 'not_enough_data',
            need: 'table_records',
        })
        expect((await context.recommend.view({ language: 'ko', userId: SCORE_USER }))?.notice).toEqual({ need: 'table_records' })
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('후보 밖 md5·중복·모르는 focus_key 는 버리고 남은 추천만 저장하며, 다시 열면 현재 결과로 보인다', async () => {
        const context = setupScoreFeatures({
            records: RECORDS,
            output: {
                summary: '  ★3 에 들어섰습니다.  ',
                picks: [
                    { md5: md5Of(300), reason: '거의 깼습니다', focus_key: `chart.${md5Of(300)}` },
                    { md5: md5Of(300), reason: '중복', focus_key: 'frontier' },
                    { md5: md5Of(999), reason: '후보 밖', focus_key: 'frontier' },
                    { md5: md5Of(301), reason: '키 없음', focus_key: 'seg.s01.bp' },
                    { md5: md5Of(302), reason: '경계 레벨', focus_key: 'frontier' },
                ],
            },
        })
        await registerScoreKey(context)
        const result = await context.recommend.interpret({ language: 'ko', userId: SCORE_USER })
        expect(result).toMatchObject({
            ok: true,
            value: {
                cached: false,
                is_current: true,
                usage: FAKE_USAGE,
                output: {
                    summary: '★3 에 들어섰습니다.',
                    picks: [
                        { md5: md5Of(300), focus_key: `chart.${md5Of(300)}` },
                        { md5: md5Of(302), focus_key: 'frontier' },
                    ],
                },
            },
        })
        const [call] = context.anthropic.generateCalls
        expect(call?.system).toBe(buildAiRecommendSystemPrompt('ko'))
        expect(call?.system).toContain(DIGEST_IS_DATA_RULE)
        expect(JSON.parse(call?.input ?? '{}')).toMatchObject({ frontier: { key: 'frontier', level: '3' }, table: { id: 'insane1' } })
        const stored = await context.recommend.view({ language: 'ko', userId: SCORE_USER })
        expect(stored?.ai).toMatchObject({ cached: true, is_current: true, output: { picks: [{ md5: md5Of(300) }, { md5: md5Of(302) }] } })
        expect([...context.memory.results.values()].map((row) => row.feature)).toEqual([AI_RECOMMEND_FEATURE])

        context.state.records = { ...RECORDS, [md5Of(303)]: recordOf('Easy') }
        expect((await context.recommend.view({ language: 'ko', userId: SCORE_USER }))?.ai).toMatchObject({ is_current: false })
        expect(context.anthropic.generateCalls).toHaveLength(1)
    })

    test('남는 추천이 없으면 출력 형식 실패로 끝나고 저장하지 않는다', async () => {
        const context = setupScoreFeatures({
            records: RECORDS,
            output: { summary: 's', picks: [{ md5: md5Of(999), reason: 'x', focus_key: 'frontier' }] },
        })
        await registerScoreKey(context)
        expect(await context.recommend.interpret({ language: 'ko', userId: SCORE_USER })).toEqual({
            ok: false,
            reason: 'output_invalid',
            provider: 'anthropic',
        })
        expect(context.memory.results.size).toBe(0)
    })

    test('refine: 이유가 한 줄을 넘거나 비어 있으면 그 추천을 버린다', () => {
        const digest = { frontier: { key: 'frontier' }, candidates: [{ key: `chart.${md5Of(1)}`, md5: md5Of(1) }] }
        expect(refineRecommendOutput({ summary: '', picks: [{ md5: md5Of(1), reason: '줄\n바꿈', focus_key: 'frontier' }] }, digest)).toBeNull()
        expect(refineRecommendOutput({ summary: '', picks: [{ md5: md5Of(1), reason: '  ', focus_key: 'frontier' }] }, digest)).toBeNull()
    })

    test('refine: 검증을 통과하지 못한 앞의 추천이 같은 곡의 통과한 뒤 추천을 막지 않고, 통과한 것만 중복으로 센다', () => {
        const digest = { frontier: { key: 'frontier' }, candidates: [{ key: `chart.${md5Of(1)}`, md5: md5Of(1) }] }
        const refined = refineRecommendOutput(
            {
                summary: '',
                picks: [
                    { md5: md5Of(1), reason: '모르는 키', focus_key: 'seg.s01.bp' },
                    { md5: md5Of(1), reason: '맞는 키', focus_key: 'frontier' },
                    { md5: md5Of(1), reason: '다시', focus_key: 'frontier' },
                ],
            },
            digest,
        )
        expect(refined?.picks).toEqual([{ md5: md5Of(1), reason: '맞는 키', focus_key: 'frontier' }])
    })
})

describe('AI 연습 폴더 (기능 6)', () => {
    const PREVIEW = {
        name_suffix: '★3 실패곡',
        description: '거의 깬 곡부터',
        items: [
            { md5: md5Of(300), memo: '후반 집중' },
            { md5: md5Of(999), memo: '후보 밖' },
            { md5: md5Of(301), memo: '' },
        ],
    }

    test('미리보기는 후보 안 곡만 남기고 폴더를 만들지 않는다', async () => {
        const context = setupScoreFeatures({ records: RECORDS, output: PREVIEW })
        await registerScoreKey(context)
        const preview = await context.practice.preview({ language: 'ko', userId: SCORE_USER, input: { source: 'recommend' } })
        expect(preview).toMatchObject({
            ok: true,
            value: { output: { name_suffix: '★3 실패곡', items: [{ md5: md5Of(300) }, { md5: md5Of(301), memo: '' }] } },
        })
        expect(await context.folderService.list(SCORE_USER)).toEqual([])
        expect(JSON.parse(context.anthropic.generateCalls[0]?.input ?? '{}')).toMatchObject({
            source: { kind: 'recommend' },
            limits: { max_items: 30 },
        })
        expect(context.anthropic.generateCalls[0]?.system).toBe(buildAiPracticeSystemPrompt('ko'))
    })

    test('만들기를 누를 때만 폴더가 생기고, 같은 미리보기로 다시 누르면 같은 폴더를 갱신한다', async () => {
        const context = setupScoreFeatures({ records: RECORDS })
        const input = { source: 'recommend' as const, ...PREVIEW }
        const first = await context.practice.create({ userId: SCORE_USER, input })
        expect(first).toMatchObject({ ok: true, value: { outcome: 'created', name: 'AI 연습 · ★3 실패곡', item_count: 2 } })
        const second = await context.practice.create({ userId: SCORE_USER, input })
        expect(second).toMatchObject({ ok: true, value: { outcome: 'updated', item_count: 2 } })
        const folders = await context.folderService.list(SCORE_USER)
        expect(folders).toHaveLength(1)
        const detail = await context.folderService.get({ userId: SCORE_USER, folderId: folders[0]?.id ?? '' })
        expect(detail?.items.map((item) => [item.md5, item.note, item.title])).toEqual([
            [md5Of(300), '후반 집중', 'L3-0'],
            [md5Of(301), '', 'L3-1'],
        ])
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('같은 미리보기로 동시에 만들기를 눌러도 폴더는 하나만 남고 하나는 created, 하나는 updated 이며 링크는 남은 폴더를 가리킨다', async () => {
        const context = setupScoreFeatures({ records: RECORDS })
        const input = { source: 'recommend' as const, ...PREVIEW }
        const results = await Promise.all([
            context.practice.create({ userId: SCORE_USER, input }),
            context.practice.create({ userId: SCORE_USER, input }),
        ])
        expect(results.map((result) => (result.ok ? result.value.outcome : result.reason)).toSorted()).toEqual(['created', 'updated'])
        const folders = await context.folderService.list(SCORE_USER)
        expect(folders).toHaveLength(1)
        expect(folders[0]).toMatchObject({ name: 'AI 연습 · ★3 실패곡', item_count: 2 })
        expect([...context.practiceMemory.links.values()]).toEqual([folders[0]?.id])
        expect(results.map((result) => (result.ok ? result.value.folder_id : ''))).toEqual([folders[0]?.id, folders[0]?.id])
    })

    test('동시 생성에서 진 쪽이 폴더 한도에 걸려도 실패로 답하지 않고 이긴 쪽 폴더를 갱신한다', async () => {
        const context = setupScoreFeatures({ records: RECORDS })
        for (const index of Array.from({ length: FOLDER_LIMIT.FOLDERS_PER_USER - 1 }, (_, value) => value)) {
            await context.folderService.create({ userId: SCORE_USER, input: { name: `f${index}`, description: '' } })
        }
        const input = { source: 'recommend' as const, ...PREVIEW }
        const results = await Promise.all([
            context.practice.create({ userId: SCORE_USER, input }),
            context.practice.create({ userId: SCORE_USER, input }),
        ])
        expect(results.map((result) => (result.ok ? result.value.outcome : result.reason)).toSorted()).toEqual(['created', 'updated'])
        expect(await context.folderService.list(SCORE_USER)).toHaveLength(FOLDER_LIMIT.FOLDERS_PER_USER)
    })

    test('이름이 겹치면 끝에 오늘 날짜(UTC)를 붙이고, 후보 안 곡이 없으면 만들지 않는다', async () => {
        const context = setupScoreFeatures({ records: RECORDS })
        await context.folderService.create({ userId: SCORE_USER, input: { name: 'AI 연습 · ★3 실패곡', description: '' } })
        const created = await context.practice.create({ userId: SCORE_USER, input: { source: 'recommend', ...PREVIEW } })
        expect(created).toMatchObject({ ok: true, value: { name: `AI 연습 · ★3 실패곡 ${new Date(SCORE_NOW).toISOString().slice(0, 10)}` } })
        const none = await context.practice.create({
            userId: SCORE_USER,
            input: { source: 'recommend', ...PREVIEW, items: [{ md5: md5Of(999), memo: '' }] },
        })
        expect(none).toEqual({ ok: false, reason: 'no_items' })
    })

    test('폴더가 50개면 보기에서 알리고 미리보기는 AI 를 부르지 않는다', async () => {
        const context = setupScoreFeatures({ records: RECORDS, output: PREVIEW })
        await registerScoreKey(context)
        for (const index of Array.from({ length: FOLDER_LIMIT.FOLDERS_PER_USER }, (_, value) => value)) {
            await context.folderService.create({ userId: SCORE_USER, input: { name: `f${index}`, description: '' } })
        }
        const view = await context.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'recommend' } })
        expect(view).toMatchObject({ ok: true, value: { folders: { used: 50, limit: 50, can_create: false } } })
        expect(await context.practice.preview({ language: 'ko', userId: SCORE_USER, input: { source: 'recommend' } })).toEqual({
            ok: false,
            reason: 'folder_limit',
            used: 50,
            limit: 50,
        })
        expect(await context.practice.create({ userId: SCORE_USER, input: { source: 'recommend', ...PREVIEW } })).toMatchObject({
            ok: false,
            reason: 'limit_exceeded',
        })
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('태그 출처는 곡별 패턴이 없으면 안내하고, 있으면 태그 비율 순으로 후보를 고른다', async () => {
        const noPattern = setupScoreFeatures({ records: RECORDS })
        expect(await noPattern.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'tag', tag: 'trill' } })).toMatchObject({
            ok: true,
            value: { notice: { need: 'chart_patterns' }, candidates: [] },
        })
        expect(await noPattern.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'tag', tag: 'nope' } })).toEqual({
            ok: false,
            reason: 'invalid_source',
            field: 'tag',
        })
        const tags = new Map([
            [md5Of(200), { trill: 0.15 }],
            [md5Of(301), { trill: 0.4 }],
            [md5Of(400), { trill: 0.25 }],
            [md5Of(401), { trill: 0.05 }],
        ])
        const withPattern = setupScoreFeatures({ records: RECORDS, tags })
        const view = await withPattern.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'tag', tag: 'trill' } })
        expect(view.ok && view.value.candidates.map((candidate) => candidate.md5)).toEqual([md5Of(301), md5Of(400), md5Of(200)])
    })

    test('refine: 메모가 100자를 넘는 항목은 자르지 않고 버리며 이름이 맞지 않으면 기본 이름을 쓴다', () => {
        const refined = refinePracticeOutput(
            {
                name_suffix: 'x'.repeat(41),
                description: 'd'.repeat(256),
                items: [
                    { md5: md5Of(1), memo: 'm'.repeat(101) },
                    { md5: md5Of(2), memo: 'ok' },
                ],
            },
            { candidates: [{ md5: md5Of(1) }, { md5: md5Of(2) }], source: 'tag', language: 'ko' },
        )
        expect(refined).toEqual({ name_suffix: '패턴 연습', description: '', items: [{ md5: md5Of(2), memo: 'ok' }] })
    })

    test('refine: 메모가 맞지 않아 버려진 앞 항목이 같은 곡의 맞는 뒤 항목을 막지 않는다', () => {
        const refined = refinePracticeOutput(
            {
                name_suffix: '연습',
                description: '',
                items: [
                    { md5: md5Of(1), memo: 'm'.repeat(101) },
                    { md5: md5Of(1), memo: '맞는 메모' },
                    { md5: md5Of(1), memo: '또' },
                ],
            },
            { candidates: [{ md5: md5Of(1) }], source: 'tag', language: 'ko' },
        )
        expect(refined?.items).toEqual([{ md5: md5Of(1), memo: '맞는 메모' }])
    })
})

describe('목표 계획 (기능 7)', () => {
    test('목표는 표·레벨·램프·곡 수를 검사하고 활성 목표는 10개까지다', async () => {
        const context = setupScoreFeatures({ records: RECORDS })
        const create = (input: Parameters<typeof context.goals.create>[0]['input']) => context.goals.create({ userId: SCORE_USER, input })
        expect(await create({ kind: 'clear_all_level', table: 'other', level: '3' })).toEqual({ ok: false, reason: 'table_not_found' })
        expect(await create({ kind: 'clear_all_level', table: 'insane1', level: '99' })).toEqual({
            ok: false,
            reason: 'invalid_goal',
            field: 'level',
        })
        expect(await create({ kind: 'lamp_count_at_level', table: 'insane1', level: '3', lamp: 'Failed', count: 1 })).toEqual({
            ok: false,
            reason: 'invalid_goal',
            field: 'lamp',
        })
        expect(await create({ kind: 'lamp_count_at_level', table: 'insane1', level: '3', lamp: 'Hard', count: 5 })).toEqual({
            ok: false,
            reason: 'invalid_goal',
            field: 'count',
        })
        expect(await create({ kind: 'ex_rate_on_chart', table: 'insane1', md5: md5Of(999), ex_rate: 90 })).toEqual({
            ok: false,
            reason: 'invalid_goal',
            field: 'md5',
        })
        for (const level of Array.from({ length: 10 }, () => '3'))
            expect((await create({ kind: 'clear_all_level', table: 'insane1', level })).ok).toBe(true)
        expect(await create({ kind: 'clear_all_level', table: 'insane1', level: '3' })).toEqual({ ok: false, reason: 'goal_limit', limit: 10 })
        const [first] = await context.goals.list(SCORE_USER)
        expect(await context.goals.archive({ userId: SCORE_USER, goalId: first?.id ?? '' })).toBe(true)
        expect(await context.goals.archive({ userId: 'user-bob', goalId: first?.id ?? '' })).toBe(false)
        expect(await context.goals.list(SCORE_USER)).toHaveLength(9)
    })

    test('진행도는 결정론으로 계산하고 남은 곡은 목표에 가까운 순이다', () => {
        const records = {
            ...levelRecords(table, { '2': 'Easy' }),
            [md5Of(200)]: recordOf('Hard', 90, SCORE_NOW - 3 * 86_400_000),
            [md5Of(201)]: recordOf('Failed', 70),
        }
        const { charts } = readTableProgress(table, records)
        const lamp = computeGoalProgress(
            { kind: 'lamp_count_at_level', table: 'insane1', level: '2', lamp: 'Hard', count: 3 },
            table,
            charts,
            SCORE_NOW,
        )
        expect(lamp.progress).toEqual({ label: '★2 에서 Hard 이상 3곡', target: 3, achieved: 1, done: false, remaining: 2, recent_updates: 4 })
        expect(lamp.remainingCharts.map((chart) => chart.md5)).toEqual([md5Of(202), md5Of(203), md5Of(201)])
        const graded = readTableProgress(table, {
            [md5Of(200)]: recordOf('Hard'),
            [md5Of(201)]: { ...recordOf('LightAssistEasy', 70), assist_grade: 1 as const },
            [md5Of(202)]: { ...recordOf('LightAssistEasy', 70), assist_grade: 4 as const },
            [md5Of(203)]: { ...recordOf('LightAssistEasy', 70), assist_grade: 0 as const },
        })
        const behindGrade = computeGoalProgress(
            { kind: 'lamp_count_at_level', table: 'insane1', level: '2', lamp: 'Hard', count: 4 },
            table,
            graded.charts,
            SCORE_NOW,
        )
        expect(behindGrade.remainingCharts.map((chart) => [chart.md5, chart.assistGrade])).toEqual([
            [md5Of(202), 4],
            [md5Of(201), 1],
            [md5Of(203), 0],
        ])
        const clearAll = computeGoalProgress({ kind: 'clear_all_level', table: 'insane1', level: '2' }, table, charts, SCORE_NOW)
        expect(clearAll.progress).toMatchObject({ target: 4, achieved: 3, remaining: 1, done: false })
        const ex = computeGoalProgress({ kind: 'ex_rate_on_chart', table: 'insane1', md5: md5Of(200), ex_rate: 90 }, table, charts, SCORE_NOW)
        expect(ex.progress).toMatchObject({ label: 'L2-0 EX 90%', achieved: 90, done: true, remaining: 0 })
        expect(ex.remainingCharts).toEqual([])
    })

    test('계획은 남은 곡 밖 md5 와 주차 범위 밖 단계를 버리고, 이미 이룬 목표는 AI 를 부르지 않는다', async () => {
        const context = setupScoreFeatures({
            records: RECORDS,
            output: {
                summary: '요약',
                steps: [
                    { week_index: 2, focus: '둘째 주', chart_md5s: [md5Of(301), md5Of(999), md5Of(301)], check_key: 'goal' },
                    { week_index: 1, focus: '첫 주', chart_md5s: [md5Of(300)], check_key: `chart.${md5Of(300)}` },
                    { week_index: 9, focus: '범위 밖', chart_md5s: [], check_key: 'goal' },
                    { week_index: 3, focus: '키 없음', chart_md5s: [], check_key: 'week.plays' },
                ],
            },
        })
        await registerScoreKey(context)
        const created = await context.goals.create({ userId: SCORE_USER, input: { kind: 'clear_all_level', table: 'insane1', level: '3' } })
        const goalId = created.ok ? created.value.id : ''
        const plan = await context.goals.interpret({ language: 'ko', userId: SCORE_USER, goalId })
        expect(plan).toMatchObject({
            ok: true,
            value: {
                output: {
                    steps: [
                        { week_index: 1, chart_md5s: [md5Of(300)] },
                        { week_index: 2, chart_md5s: [md5Of(301)] },
                    ],
                },
            },
        })
        expect(context.anthropic.generateCalls[0]?.system).toBe(buildAiGoalSystemPrompt('ko'))
        expect((await context.goals.planView({ language: 'ko', userId: SCORE_USER, goalId }))?.ai).toMatchObject({ is_current: true })
        const reached = await context.goals.create({ userId: SCORE_USER, input: { kind: 'clear_all_level', table: 'insane1', level: '1' } })
        expect(await context.goals.interpret({ language: 'ko', userId: SCORE_USER, goalId: reached.ok ? reached.value.id : '' })).toEqual({
            ok: false,
            reason: 'not_enough_data',
            need: 'goal_reached',
        })
        expect(await context.goals.interpret({ language: 'ko', userId: 'user-bob', goalId })).toBeNull()
        expect(context.anthropic.generateCalls).toHaveLength(1)
    })

    test('목표 출처의 연습 폴더 후보는 그 목표의 남은 곡이다', async () => {
        const context = setupScoreFeatures({ records: RECORDS })
        const created = await context.goals.create({ userId: SCORE_USER, input: { kind: 'clear_all_level', table: 'insane1', level: '3' } })
        const goalId = created.ok ? created.value.id : ''
        const view = await context.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'goal', goal_id: goalId } })
        expect(view.ok && view.value.candidates.map((candidate) => candidate.md5)).toEqual([md5Of(300), md5Of(301), md5Of(302), md5Of(303)])
        expect(await context.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'goal' } })).toEqual({
            ok: false,
            reason: 'invalid_source',
            field: 'goal_id',
        })
    })

    test('refine: 단계가 하나도 남지 않으면 버린다', () => {
        expect(
            refineGoalPlanOutput({ summary: 's', steps: [{ week_index: 0, focus: 'f', chart_md5s: [], check_key: 'goal' }] }, { remaining: [] }),
        ).toBeNull()
    })

    test('refine: 같은 주차가 둘이면 검증을 통과한 앞의 것만 남기고, 안 맞는 앞 단계는 뒤의 맞는 같은 주차를 막지 않는다', () => {
        const digest = { goal: { key: 'goal' }, remaining: [{ key: `chart.${md5Of(1)}`, md5: md5Of(1) }] }
        const refined = refineGoalPlanOutput(
            {
                summary: 's',
                steps: [
                    { week_index: 2, focus: '모르는 키', chart_md5s: [], check_key: 'seg.s01.bp' },
                    { week_index: 2, focus: '둘째 주 첫째', chart_md5s: [md5Of(1)], check_key: 'goal' },
                    { week_index: 1, focus: '첫째 주', chart_md5s: [], check_key: 'goal' },
                    { week_index: 2, focus: '둘째 주 중복', chart_md5s: [], check_key: 'goal' },
                ],
            },
            digest,
        )
        const steps = refined?.steps ?? []
        expect(steps.map((step) => [step.week_index, step.focus])).toEqual([
            [1, '첫째 주'],
            [2, '둘째 주 첫째'],
        ])
        expect(new Set(steps.map((step) => step.week_index)).size).toBe(steps.length)
    })
})

const BASE_FEATURES = computeChartFeatures(
    makePatternDocument([
        [0, 1, 0],
        [250_000, 2, 0],
        [500_000, 3, 0],
    ]),
)

/** Current (ANALYSIS_VERSION 2) features of a chart with these tag shares and 90th-percentile NPS. */
const featuresWith = (patch: { tags?: Record<string, number>; p90?: number }): ChartFeatures => ({
    ...BASE_FEATURES,
    tags: Object.fromEntries(Object.entries(patch.tags ?? {}).map(([tag, ratio]) => [tag, { notes: 1, ratio }])),
    nps: { ...BASE_FEATURES.nps, p90_2s: patch.p90 ?? 0 },
})

const FEATURES = new Map([
    [md5Of(201), featuresWith({ tags: { trill: 0.15 } })],
    [md5Of(301), featuresWith({ tags: { trill: 0.3 } })],
    [md5Of(302), featuresWith({ p90: 12.5 })],
    [md5Of(303), featuresWith({ p90: 16 })],
    [md5Of(401), featuresWith({ tags: { trill: 0.05 } })],
])

const cellOf = (key: string, kind: string, value: number): WeaknessProfileCell => ({
    key,
    kind,
    value,
    unit: 'ex_residual',
    ex_ratio: 0.8,
    bp_rate: 0.05,
    timing: null,
    notes: 900,
    charts: 12,
    plays: 30,
    auto_plays: 0,
    p_value: 0.001,
    q_threshold: 0.01,
    evidence: 'strong',
    leaning: 'weakness',
    sample_level: 'medium',
    trend_delta: null,
    interaction: null,
})

/** A profile whose matchable weaknesses are `tag:trill` and `density:limit` (12 NPS); `lane:3` cannot pick charts, `tag:jack` is uncertain. */
const profileOf = (trillValue = -0.06): WeaknessProfileView => ({
    ...trainingProfileOf({
        weaknesses: [
            trainingWeaknessOf('tag:trill', 'tag', { value: trillValue }),
            trainingWeaknessOf('density:limit', 'density', { unit: 'nps', value: 12 }),
            trainingWeaknessOf('lane:3', 'lane', { selectable_for_folder: false }),
            trainingWeaknessOf('tag:jack', 'tag', { evidence: 'uncertain' }),
            trainingWeaknessOf('timing:trill', 'timing', { unit: 'ms', value: 6, selectable_for_folder: false }),
        ],
    }),
    cells: [cellOf('tag:trill', 'tag', trillValue)],
})

/** Features 5, 6 and 7 over the real chart-tag loader (current or stale features) with a weakness profile, or none. */
const setupWithProfile = (options: { stale?: readonly string[]; profile?: WeaknessProfileView | null; output?: unknown } = {}) => {
    const context = setupScoreFeatures({ records: RECORDS, output: options.output })
    const state = { profile: options.profile === undefined ? profileOf() : options.profile }
    const loadChartTags = createChartTagLoader({
        readFeaturesMany: async (md5s) =>
            new Map(
                [...FEATURES]
                    .filter(([md5]) => md5s.includes(md5))
                    .map(([md5, features]) => [md5, { features, isStale: options.stale?.includes(md5) ?? false }] as const),
            ),
    })
    const tableDeps: TableFeatureDeps = {
        loadTable: (tableId) => (tableId === undefined || tableId === context.table.id ? context.table : null),
        loadRecords: async () => context.state.records,
        loadChartTags,
    }
    const shared = { runner: context.aiFeatureRunner, results: context.aiResultService }
    const profileDeps = state.profile === null ? {} : { profile: { view: async () => state.profile ?? profileOf() } }
    let counter = 0
    const newId = (prefix: string) => `${prefix}${(++counter).toString(16).padStart(30, 'a')}`
    const now = () => SCORE_NOW
    const recommend = createAiRecommendService({ ...tableDeps, ...shared, ...profileDeps })
    const goals = createAiGoalService({ ...tableDeps, ...shared, ...profileDeps, db: context.goalMemory.db, newId, now })
    const practice = createAiPracticeFolderService({
        ...tableDeps,
        ...shared,
        ...profileDeps,
        db: context.practiceMemory.db,
        recommend,
        goals,
        folders: context.folderService,
        now,
    })
    return { ...context, state, recommend, goals, practice, loadChartTags }
}

describe('약점 프로필 키의 기계적 연결 (기획 2026-11-04 2.6 기능 5·6·7)', () => {
    test('추천 점수는 약점 일치 × 1.0 을 더하고 밀도 한계 × 1.25 를 넘는 곡에서 1.0 을 빼며, 후보마다 focus_keys 를 붙인다', async () => {
        const context = setupWithProfile()
        const view = await context.recommend.view({ language: 'ko', userId: SCORE_USER })
        const byMd5 = new Map((view?.candidates ?? []).map((candidate) => [candidate.md5, candidate]))
        expect([301, 302, 303].map((index) => [byMd5.get(md5Of(index))?.score, byMd5.get(md5Of(index))?.focus_keys])).toEqual([
            [2.333, ['tag:trill']],
            [2.333, ['density:limit']],
            [1, []],
        ])
        expect(byMd5.get(md5Of(300))).toMatchObject({ kind: 'failed', score: 3.85, focus_keys: [] })
        expect(view?.candidates.map((candidate) => candidate.md5).slice(0, 4)).toEqual([md5Of(300), md5Of(301), md5Of(302), md5Of(303)])
        expect(view?.profile_keys).toEqual(['tag:trill', 'density:limit'])
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('v1·낡은 특징(새 필드 없음, isStale)이면 던지지 않고 밀도 일치·감점은 0, 태그 일치는 그대로 쓴다', async () => {
        const context = setupWithProfile({ stale: [md5Of(301), md5Of(302), md5Of(303)] })
        const load = await context.loadChartTags([md5Of(301), md5Of(302)])
        expect(load.extras?.size).toBe(0)
        expect([...load.staleMd5s].toSorted()).toEqual([md5Of(301), md5Of(302)])
        const view = await context.recommend.view({ language: 'ko', userId: SCORE_USER })
        const scores = new Map((view?.candidates ?? []).map((candidate) => [candidate.md5, [candidate.score, candidate.focus_keys]]))
        expect([301, 302, 303].map((index) => scores.get(md5Of(index)))).toEqual([
            [2.333, ['tag:trill']],
            [2, []],
            [2, []],
        ])
        expect(view?.pattern_coverage).toMatchObject({ stale_features: 3 })
        const density = await context.practice.view({
            language: 'ko',
            userId: SCORE_USER,
            input: { source: 'weakness', focus_key: 'density:limit' },
        })
        expect(density).toMatchObject({ ok: true, value: { candidates: [], notice: { need: 'candidates' } } })
    })

    test('프로필이 없으면 추천은 약점 없이 예전 점수 그대로다', async () => {
        const context = setupWithProfile({ profile: null })
        const view = await context.recommend.view({ language: 'ko', userId: SCORE_USER })
        expect(view?.candidates.find((candidate) => candidate.md5 === md5Of(301))).toMatchObject({ score: 2, focus_keys: [] })
        expect(view?.profile_keys).toEqual([])
    })

    test('refine: 약점 키 focus_key 는 그 후보의 focus_keys 일 때만 남는다', () => {
        const digest = {
            frontier: { key: 'frontier' },
            weaknesses: [{ key: 'tag:trill' }],
            candidates: [
                { key: `chart.${md5Of(1)}`, md5: md5Of(1), focus_keys: ['tag:trill'] },
                { key: `chart.${md5Of(2)}`, md5: md5Of(2), focus_keys: [] },
            ],
        }
        const refined = refineRecommendOutput(
            {
                summary: '',
                picks: [
                    { md5: md5Of(2), reason: '트릴', focus_key: 'tag:trill' },
                    { md5: md5Of(1), reason: '트릴', focus_key: 'tag:trill' },
                    { md5: md5Of(2), reason: '경계', focus_key: 'frontier' },
                ],
            },
            digest,
        )
        expect(refined?.picks.map((pick) => [pick.md5, pick.focus_key])).toEqual([
            [md5Of(1), 'tag:trill'],
            [md5Of(2), 'frontier'],
        ])
    })

    test('연습 폴더 weakness 출처는 그 약점의 몫이 큰 곡 순이고, AI 에 약점을 함께 넘긴다', async () => {
        const preview = { name_suffix: '트릴', description: '', items: [{ md5: md5Of(301), memo: '' }] }
        const context = setupWithProfile({ output: preview })
        await registerScoreKey(context)
        const input = { source: 'weakness' as const, focus_key: 'tag:trill' }
        const view = await context.practice.view({ language: 'ko', userId: SCORE_USER, input })
        expect(view).toMatchObject({ ok: true, value: { source: 'weakness', focus_key: 'tag:trill', tag: null, notice: null } })
        expect(view.ok && view.value.candidates.map((candidate) => candidate.md5)).toEqual([md5Of(301), md5Of(201)])
        const density = await context.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'weakness', focus_key: 'density:limit' } })
        expect(density.ok && density.value.candidates.map((candidate) => candidate.md5)).toEqual([md5Of(302)])
        expect(await context.practice.preview({ language: 'ko', userId: SCORE_USER, input })).toMatchObject({ ok: true })
        expect(JSON.parse(context.anthropic.generateCalls[0]?.input ?? '{}')).toMatchObject({
            source: { kind: 'weakness', focus_tag: null, weakness: { key: 'tag:trill', kind: 'tag', unit: 'ex_residual' } },
        })
        const created = await context.practice.create({ userId: SCORE_USER, input: { ...input, ...preview } })
        expect(created).toMatchObject({ ok: true, value: { outcome: 'created', name: 'AI 연습 · 트릴', item_count: 1 } })
    })

    test('프로필 weaknesses 에 없는 키는 weakness_profile, 곡을 고를 수 없는 키는 candidates 로 공급자 호출 전에 막는다', async () => {
        const context = setupWithProfile({ output: { name_suffix: 'x', description: '', items: [] } })
        await registerScoreKey(context)
        for (const [focus_key, need] of [
            ['tag:stair', 'weakness_profile'],
            ['lane:3', 'candidates'],
        ] as const) {
            const input = { source: 'weakness' as const, focus_key }
            const expected = { ok: false, reason: 'not_enough_data', need } as const
            expect(await context.practice.view({ language: 'ko', userId: SCORE_USER, input })).toEqual(expected)
            expect(await context.practice.preview({ language: 'ko', userId: SCORE_USER, input })).toEqual(expected)
            expect(
                await context.practice.create({
                    userId: SCORE_USER,
                    input: { ...input, name_suffix: 'x', description: '', items: [{ md5: md5Of(301), memo: '' }] },
                }),
            ).toEqual(expected)
        }
        expect(await context.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'weakness' } })).toEqual({
            ok: false,
            reason: 'invalid_source',
            field: 'focus_key',
        })
        const noProfile = setupWithProfile({ profile: null })
        expect(await noProfile.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'weakness', focus_key: 'tag:trill' } })).toEqual({
            ok: false,
            reason: 'not_enough_data',
            need: 'weakness_profile',
        })
        expect(context.anthropic.generateCalls).toHaveLength(0)
        expect(await context.folderService.list(SCORE_USER)).toEqual([])
    })

    test('weakness_residual 목표는 프로필 약점 키만 받고, 생성 때 기준값을 남겨 지금 값과의 비로 진행한다', async () => {
        const context = setupWithProfile()
        const create = (input: Parameters<typeof context.goals.create>[0]['input']) => context.goals.create({ userId: SCORE_USER, input })
        expect(await create({ kind: 'weakness_residual', table: 'insane1', key: 'tag:stair', target_residual: -0.02 })).toEqual({
            ok: false,
            reason: 'not_enough_data',
            need: 'weakness_profile',
        })
        expect(await create({ kind: 'weakness_residual', table: 'insane1', key: 'timing:trill', target_residual: -0.02 })).toEqual({
            ok: false,
            reason: 'invalid_goal',
            field: 'key',
        })
        expect(context.goalMemory.goals.size).toBe(0)
        const created = await create({ kind: 'weakness_residual', table: 'insane1', key: 'tag:trill', target_residual: -0.02 })
        expect(created).toMatchObject({
            ok: true,
            value: {
                params: { kind: 'weakness_residual', key: 'tag:trill', target_residual: -0.02, baseline: -0.06 },
                progress: { target: -0.02, achieved: -0.06, current: -0.06, ratio: 0, done: false, remaining: 1 },
            },
        })
        context.state.profile = profileOf(-0.04)
        expect((await context.goals.list(SCORE_USER))[0]?.progress).toMatchObject({ current: -0.04, ratio: 0.5, done: false })
        const goalId = created.ok ? created.value.id : ''
        const plan = await context.goals.planView({ language: 'ko', userId: SCORE_USER, goalId })
        expect(plan?.remaining.map((chart) => chart.md5)).toEqual([md5Of(301), md5Of(201)])
        context.state.profile = profileOf(-0.01)
        expect((await context.goals.list(SCORE_USER))[0]?.progress).toMatchObject({ current: -0.01, ratio: 1, done: true, remaining: 0 })
    })

    test('density_limit 목표는 지금 EX 밀도 한계를 기준값으로 두고 p90 이 목표 ±1.5 인 곡으로 계획하며, 프로필이 없으면 만들지 않는다', async () => {
        const context = setupWithProfile()
        const created = await context.goals.create({ userId: SCORE_USER, input: { kind: 'density_limit', table: 'insane1', target_nps: 14 } })
        expect(created).toMatchObject({
            ok: true,
            value: { params: { kind: 'density_limit', target_nps: 14, baseline: 12 }, progress: { target: 14, current: 12, ratio: 0, done: false } },
        })
        const goalId = created.ok ? created.value.id : ''
        expect((await context.goals.planView({ language: 'ko', userId: SCORE_USER, goalId }))?.remaining.map((chart) => chart.md5)).toEqual([
            md5Of(302),
        ])
        const folder = await context.practice.view({ language: 'ko', userId: SCORE_USER, input: { source: 'goal', goal_id: goalId } })
        expect(folder.ok && folder.value.candidates.map((candidate) => candidate.md5)).toEqual([md5Of(302)])

        const stale = setupWithProfile({ stale: [md5Of(302)] })
        const staleGoal = await stale.goals.create({ userId: SCORE_USER, input: { kind: 'density_limit', table: 'insane1', target_nps: 14 } })
        const stalePlan = await stale.goals.planView({ language: 'ko', userId: SCORE_USER, goalId: staleGoal.ok ? staleGoal.value.id : '' })
        expect(stalePlan).toMatchObject({ remaining: [], notice: { need: 'remaining_charts' } })

        const noProfile = setupWithProfile({ profile: null })
        expect(await noProfile.goals.create({ userId: SCORE_USER, input: { kind: 'density_limit', table: 'insane1', target_nps: 14 } })).toEqual({
            ok: false,
            reason: 'not_enough_data',
            need: 'weakness_profile',
        })
    })
})

describe('어시스트 등급 프롬프트', () => {
    test('등급이 든 digest 를 받는 기능의 프롬프트는 같은 등급 규칙 문장을 싣는다', () => {
        for (const prompt of [buildAiRecommendSystemPrompt('ko'), buildAiGoalSystemPrompt('ko'), buildAiPracticeSystemPrompt('ko')])
            expect(prompt).toContain(ASSIST_GRADE_RULE)
        expect(ASSIST_GRADE_RULE).toContain('1 (ASSIST EASY), 2 (ASSIST CLEAR), 3 (ASSIST HARD), 4 (ASSIST EXHARD)')
        expect(ASSIST_GRADE_RULE).toContain('LightAssistEasy')
    })
})
