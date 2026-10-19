import { describe, expect, test } from 'bun:test'
import { clearLampToId } from '@server/dto/common'
import {
    AI_JUDGE_HABIT_SYSTEM_PROMPT,
    computeJudgeHabit,
    createAiJudgeHabitService,
    refineJudgeHabitOutput,
    type TimingScoreRow,
} from '@server/service/domain/ai/judge-habit.service'
import {
    createAiWeeklySummaryService,
    resolveWeekStart,
    summarizeWeek,
    weekStartOf,
    type PriorBestRow,
    type WeeklyScoreRow,
} from '@server/service/domain/ai/weekly-summary.service'
import { FAKE_USAGE, createFakeAdapter, createMemoryAiServices, testKekEnv } from '../../support/ai-memory-db'
import { SCORE_API_KEY, SCORE_USER, md5Of, registerScoreKey } from '../../support/ai-score-fixture'

const MONDAY = Date.UTC(2026, 9, 19)
const DAY = 86_400_000
const SHA = { alpha: 'a'.repeat(64), bravo: 'b'.repeat(64), charlie: 'c'.repeat(64) }

const play = (overrides: Partial<WeeklyScoreRow>): WeeklyScoreRow => ({
    chartSha256: SHA.alpha,
    chartMd5: md5Of(1),
    title: 'Alpha',
    clear: clearLampToId('Easy'),
    exScore: 1500,
    maxExScore: 2000,
    playedAt: MONDAY + DAY,
    fast: 30,
    slow: 10,
    judgedNotes: 1000,
    scratchAuto: false,
    autoOffset: false,
    meanOffsetUs: null,
    ruleVersion: null,
    ...overrides,
})

const setupWeekly = (rows: WeeklyScoreRow[], priors: PriorBestRow[], now = MONDAY + 3 * DAY, output?: unknown) => {
    const anthropic = createFakeAdapter('anthropic', {
        validKeys: new Set([SCORE_API_KEY]),
        generate: () => ({ data: output ?? {}, usage: FAKE_USAGE }),
    })
    const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => testKekEnv('v1'), now: () => now })
    const ranges: { from: number; to: number }[] = []
    const service = createAiWeeklySummaryService({
        db: {
            listScoresBetween: async ({ from, to }) => {
                ranges.push({ from, to })
                return rows.filter((row) => row.playedAt >= from && row.playedAt < to)
            },
            listPriorBests: async ({ before }) => (before === MONDAY ? priors : []),
        },
        runner: services.aiFeatureRunner,
        results: services.aiResultService,
        levelLabelOf: (md5) => (md5 === md5Of(1) ? '★12' : null),
        now: () => now,
    })
    return { ...services, anthropic, service, ranges }
}

describe('주간 경계 (UTC)', () => {
    test('주는 월요일 00:00 UTC 에 시작하고 일요일 23:59:59.999 는 앞 주다', () => {
        expect(weekStartOf(MONDAY)).toBe(MONDAY)
        expect(weekStartOf(MONDAY + 7 * DAY - 1)).toBe(MONDAY)
        expect(weekStartOf(MONDAY - 1)).toBe(MONDAY - 7 * DAY)
        expect(weekStartOf(Date.UTC(2026, 9, 25, 23, 59, 59, 999))).toBe(MONDAY)
        expect(weekStartOf(Date.UTC(2026, 9, 26, 0, 0, 0))).toBe(MONDAY + 7 * DAY)
    })

    test('요청한 주는 실제 월요일이어야 하고 이번 주보다 뒤면 안 된다', () => {
        const now = MONDAY + 2 * DAY
        expect(resolveWeekStart(undefined, now)).toBe(MONDAY)
        expect(resolveWeekStart('2026-10-12', now)).toBe(MONDAY - 7 * DAY)
        expect(resolveWeekStart('2026-10-19', now)).toBe(MONDAY)
        expect(resolveWeekStart('2026-10-20', now)).toBeNull()
        expect(resolveWeekStart('2026-10-26', now)).toBeNull()
        expect(resolveWeekStart('2026-02-30', now)).toBeNull()
    })
})

describe('주간 성장 요약 (기능 10)', () => {
    test('그 주 이전 최고와 비교해 새 램프·첫 클리어·EX 갱신·처음 친 곡을 세고 SCRATCH AUTO 는 따로 본다', () => {
        const rows = [
            play({ clear: clearLampToId('Hard'), exScore: 1700 }),
            play({ clear: clearLampToId('Failed'), exScore: 1800, playedAt: MONDAY + 2 * DAY }),
            play({ chartSha256: SHA.bravo, chartMd5: null, title: 'Bravo', clear: clearLampToId('Normal'), meanOffsetUs: 3000, ruleVersion: 4 }),
            play({
                chartSha256: SHA.charlie,
                chartMd5: md5Of(3),
                title: 'Charlie',
                clear: clearLampToId('Easy'),
                scratchAuto: true,
                meanOffsetUs: -1000,
                ruleVersion: 4,
                judgedNotes: 3000,
            }),
            play({ chartSha256: SHA.charlie, chartMd5: md5Of(3), title: 'Charlie', autoOffset: true, meanOffsetUs: 9000, ruleVersion: 4 }),
        ]
        const priors: PriorBestRow[] = [
            { chartSha256: SHA.alpha, scratchAuto: false, clear: clearLampToId('Easy'), exScore: 1600 },
            { chartSha256: SHA.bravo, scratchAuto: false, clear: clearLampToId('Failed'), exScore: 900 },
            { chartSha256: SHA.charlie, scratchAuto: false, clear: clearLampToId('Hard'), exScore: 1900 },
        ]
        const summary = summarizeWeek({ rows, priors, levelLabelOf: (md5) => (md5 === md5Of(1) ? '★12' : null) })
        expect(summary.totals).toEqual({
            plays: 5,
            charts: 3,
            active_days: 2,
            first_plays: 1,
            lamp_ups: 3,
            new_clears: 2,
            ex_ups: 2,
            fast: 150,
            slow: 50,
            fast_share: 0.75,
            mean_offset_ms: 0,
        })
        expect(summary.lamp_changes.map((change) => [change.key, change.from_lamp, change.to_lamp, change.scratch_auto])).toEqual([
            [`chart.${md5Of(1)}`, 'Easy', 'Hard', false],
            [`chart.${SHA.bravo}`, 'Failed', 'Normal', false],
            [`chart.${md5Of(3)}`, 'NoPlay', 'Easy', true],
        ])
        expect(summary.ex_changes.map((change) => [change.title, change.from_ex_rate, change.to_ex_rate])).toEqual([
            ['Bravo', 45, 75],
            ['Alpha', 80, 90],
        ])
        expect(summary.levels).toEqual([{ key: 'level.★12', level: '★12', plays: 2, lamp_ups: 1 }])
    })

    test('그 주와 앞 주를 [시작, 끝) 으로 읽고, 판이 없으면 AI 를 부르지 않으며 진행 중인 주를 알린다', async () => {
        const context = setupWeekly([play({ playedAt: MONDAY - DAY })], [])
        await registerScoreKey(context)
        const view = await context.service.view({ userId: SCORE_USER })
        expect(view).toMatchObject({
            week: { start: '2026-10-19', end: '2026-10-25', in_progress: true },
            notice: { need: 'week_plays' },
            previous: { plays: 1 },
        })
        expect(context.ranges).toEqual([
            { from: MONDAY, to: MONDAY + 7 * DAY },
            { from: MONDAY - 7 * DAY, to: MONDAY },
        ])
        expect(await context.service.interpret({ userId: SCORE_USER })).toEqual({ ok: false, reason: 'not_enough_data', need: 'week_plays' })
        expect(await context.service.view({ userId: SCORE_USER, week: '2026-10-20' })).toBeNull()
        expect(context.anthropic.generateCalls).toHaveLength(0)
    })

    test('지난 주를 열면 진행 중이 아니고, 키가 digest 에 없는 하이라이트는 버린다', async () => {
        const output = {
            headline: '한 주 요약',
            highlights: [
                { key: 'week.lamp_ups', title: '램프 갱신', body: '한 곡의 램프가 올랐습니다.', tone: 'strength' },
                { key: 'tag.trill.residual', title: '트릴', body: '근거 없음', tone: 'weakness' },
            ],
            next_week: '다음 주',
        }
        const context = setupWeekly([play({ playedAt: MONDAY - 3 * DAY, clear: clearLampToId('Hard') })], [], MONDAY + DAY, output)
        await registerScoreKey(context)
        const result = await context.service.interpret({ userId: SCORE_USER, week: '2026-10-12' })
        expect(result).toMatchObject({ ok: true, value: { output: { highlights: [{ key: 'week.lamp_ups' }] } } })
        expect(JSON.parse(context.anthropic.generateCalls[0]?.input ?? '{}')).toMatchObject({
            week: { key: 'week', start: '2026-10-12', in_progress: false },
        })
        expect((await context.service.view({ userId: SCORE_USER, week: '2026-10-12' }))?.ai).toMatchObject({ is_current: true, cached: true })
    })
})

const timing = (overrides: Partial<TimingScoreRow>): TimingScoreRow => ({
    playedAt: MONDAY,
    pgreat: 600,
    great: 300,
    good: 50,
    bad: 10,
    fast: 70,
    slow: 30,
    egr: 50,
    lgr: 20,
    egd: 15,
    lgd: 5,
    ebd: 5,
    lbd: 5,
    avgjudge: 12_000,
    meanOffsetUs: 4_400,
    ruleVersion: 4,
    autoOffset: false,
    greenNumber: 500,
    constant: false,
    exScore: 1500,
    maxExScore: 2000,
    ...overrides,
})

describe('판정 습관 진단 — 점수 행 부분 (기능 4)', () => {
    test('FAST 가 분명히 많고 치우침도 같은 쪽이면 노트를 늦게 보이도록 내리는 숫자를 제안한다', () => {
        const rows = Array.from({ length: 12 }, (_, index) => timing({ playedAt: MONDAY - index * DAY, avgjudge: index < 6 ? 10_000 : 14_000 }))
        const habit = computeJudgeHabit(rows)
        expect(habit.fast_slow).toMatchObject({ fast: 840, slow: 360, fast_share: 0.7, grade: 'strong', leaning: 'fast' })
        expect(habit.offset).toMatchObject({
            plays: 12,
            notes: 10_800,
            mean_offset_ms: 4.4,
            grade: 'strong',
            direction: 'later',
            suggested_shift_ms: -4,
        })
        expect(habit.avgjudge).toEqual({ plays: 12, older_ms: 14, newer_ms: 10, change_ms: -4, grade: 'strong' })
        expect(habit.green.latest).toEqual({ green: 300, lane_ms: 500, is_estimated: true })
        expect(habit.green.buckets).toEqual([
            { key: 'green.300', from: { green: 300, lane_ms: 500 }, to: { green: 350, lane_ms: 583 }, plays: 12, fast_share: 0.7, ex_rate: 75 },
        ])
    })

    test('trace 의 green 값이 있는 판은 그 값을 그대로 쓰고 추정이 아니며, 점수 행만 있는 판은 추정으로 남는다', () => {
        const traced = timing({ greenNumber: 500, traceGreen: { green: 361, greenMs: 602 } })
        const scoreOnly = timing({ playedAt: MONDAY - DAY, greenNumber: 500 })
        const onlyTrace = computeJudgeHabit([traced])
        expect(onlyTrace.green.latest).toEqual({ green: 361, lane_ms: 602, is_estimated: false })
        expect(onlyTrace.green.median).toEqual({ green: 361, lane_ms: 602, is_estimated: false })
        expect(onlyTrace.green.buckets.map((bucket) => bucket.key)).toEqual(['green.350'])
        const mixed = computeJudgeHabit([traced, scoreOnly])
        expect(mixed.green.latest).toEqual({ green: 361, lane_ms: 602, is_estimated: false })
        expect(mixed.green.median).toMatchObject({ is_estimated: true })
        expect(mixed.green.buckets.map((bucket) => [bucket.key, bucket.plays])).toEqual([
            ['green.300', 1],
            ['green.350', 1],
        ])
    })

    test('판이 적거나 규칙 세대 4 아래이거나 AUTO ADJUST 판이면 오프셋을 제안하지 않는다', () => {
        const few = computeJudgeHabit(Array.from({ length: 5 }, () => timing({})))
        expect([few.fast_slow.grade, few.offset.direction, few.offset.suggested_shift_ms]).toEqual(['insufficient', 'keep', null])
        const old = computeJudgeHabit(Array.from({ length: 12 }, () => timing({ ruleVersion: 3 })))
        expect(old.offset).toMatchObject({ plays: 0, mean_offset_ms: null, grade: 'insufficient', direction: 'keep' })
        expect(old.avgjudge).toMatchObject({ plays: 0, grade: 'insufficient' })
        const auto = computeJudgeHabit(Array.from({ length: 12 }, () => timing({ autoOffset: true })))
        expect(auto.offset).toMatchObject({ plays: 0, auto_adjust_plays: 12, direction: 'keep' })
        const opposite = computeJudgeHabit(Array.from({ length: 12 }, () => timing({ meanOffsetUs: -3000 })))
        expect(opposite.offset).toMatchObject({ grade: 'strong', direction: 'keep', suggested_shift_ms: null })
    })

    test('SLOW 쪽이면 올리는(노트가 먼저 오는) 숫자다', () => {
        const habit = computeJudgeHabit(Array.from({ length: 12 }, () => timing({ fast: 30, slow: 70, meanOffsetUs: -2600 })))
        expect(habit.offset).toMatchObject({ direction: 'earlier', suggested_shift_ms: 3 })
    })

    test('refine 는 오프셋 방향을 서버 결론으로 고정하고 키가 없는 메모를 버린다', () => {
        const refined = refineJudgeHabitOutput(
            {
                summary: '요약',
                offset_direction: 'earlier',
                offset_reasoning: '이유',
                green_direction: 'slower',
                green_reasoning: '녹색',
                notes: [
                    { key: 'timing.offset', title: '치우침', body: '빠릅니다', tone: 'neutral' },
                    { key: 'lane.3', title: '레인', body: '근거 없음', tone: 'weakness' },
                ],
            },
            { digest: { offset: { key: 'timing.offset' } }, direction: 'keep' },
        )
        expect(refined).toMatchObject({ offset_direction: 'keep', green_direction: 'slower', notes: [{ key: 'timing.offset' }] })
    })

    test('최근 판이 10판 아래면 AI 를 부르지 않고, 충분하면 고정 프롬프트로 한 번 부른다', async () => {
        const output = {
            summary: '빠르게 치는 편',
            offset_direction: 'earlier',
            offset_reasoning: '노트가 보이는 위치를 옮긴다',
            green_direction: 'keep',
            green_reasoning: 'GREEN 300 (500 ms) 유지',
            notes: [],
        }
        const anthropic = createFakeAdapter('anthropic', {
            validKeys: new Set([SCORE_API_KEY]),
            generate: () => ({ data: output, usage: FAKE_USAGE }),
        })
        const services = createMemoryAiServices({ adapters: { anthropic: anthropic.adapter }, env: () => testKekEnv('v1') })
        await registerScoreKey(services)
        const state = { rows: Array.from({ length: 9 }, () => timing({})) }
        const service = createAiJudgeHabitService({
            db: { listRecentTimingRows: async ({ limit }) => state.rows.slice(0, limit) },
            runner: services.aiFeatureRunner,
            results: services.aiResultService,
        })
        expect((await service.view(SCORE_USER)).notice).toEqual({ need: 'recent_plays' })
        expect(await service.interpret(SCORE_USER)).toEqual({ ok: false, reason: 'not_enough_data', need: 'recent_plays' })
        expect(anthropic.generateCalls).toHaveLength(0)
        state.rows = Array.from({ length: 12 }, () => timing({}))
        const result = await service.interpret(SCORE_USER)
        expect(result).toMatchObject({ ok: true, value: { output: { offset_direction: 'later' } } })
        expect(anthropic.generateCalls[0]?.system).toBe(AI_JUDGE_HABIT_SYSTEM_PROMPT)
        expect(AI_JUDGE_HABIT_SYSTEM_PROMPT).toContain('노트가 보이는 위치를 옮긴다')
        expect(JSON.parse(anthropic.generateCalls[0]?.input ?? '{}')).toMatchObject({ offset: { key: 'timing.offset', direction: 'later' } })
    })
})
