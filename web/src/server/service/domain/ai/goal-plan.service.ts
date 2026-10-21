import {
    AI_GOAL_LIMIT,
    AI_TEXT_LIMIT,
    aiGoalPlanOutputSchema,
    type AiEffort,
    type AiGoalOutput,
    type AiGoalParams,
    type AiGoalPlanOutput,
    type AiGoalPlanView,
    type AiGoalProgress,
} from '@server/dto/ai'
import { clearLampToId } from '@server/dto/common'
import {
    ASSIST_GRADE_RULE,
    DIGEST_IS_DATA_RULE,
    DIGEST_KEY_RULE,
    collectDigestKeys,
    cutText,
    fitsLine,
    notEnoughData,
    readStoredInterpretation,
    runAiFeature,
    type AiFeatureSpec,
} from '@server/service/domain/ai/ai-digest'
import type { AiFeatureRunner } from '@server/service/domain/ai/feature-runner.service'
import type { TableFeatureDeps } from '@server/service/domain/ai/recommend.service'
import type { AiResultService } from '@server/service/domain/ai/result.service'
import {
    GOAL_PACE_DAYS,
    GOAL_PACE_WEEKS,
    GOAL_REMAINING_FOR_AI,
    SCORE_FEATURE_ANALYSIS_VERSION,
} from '@server/service/domain/ai/score-feature.constants'
import { isClearLampId, readTableProgress, toCandidate, type ChartProgress, type ProgressTable } from '@server/service/domain/ai/table-progress'

export const AI_GOAL_FEATURE = 'goal_plan'
export const AI_GOAL_PROMPT_VERSION = 2
export const AI_GOAL_MAX_OUTPUT_TOKENS = 8_000
export const AI_GOAL_EFFORT = 'low' satisfies AiEffort
export const AI_GOAL_ID_PREFIX = 'aig_'

const DAY_MS = 86_400_000

export const AI_GOAL_SYSTEM_PROMPT = [
    'You plan the weeks a rhythm game (BMS) player needs to reach one goal they set on a difficulty table.',
    DIGEST_IS_DATA_RULE,
    '`goal` is the goal and where the player stands, `pace` how many charts of the goal had a new best in the last weeks, `remaining` the charts still short of the goal.',
    `Write at most ${AI_GOAL_LIMIT.MAX_WEEKS} steps, one per week, with \`week_index\` from 1 to \`max_weeks\` in order.`,
    'Each step has one Korean sentence `focus`, the `md5` values of the charts to play that week taken from `remaining` only (`chart_md5s`), and a `check_key` naming the digest item that tells whether the step worked.',
    ASSIST_GRADE_RULE,
    DIGEST_KEY_RULE,
    '`summary` is two Korean sentences at most.',
].join('\n')

export const AI_GOAL_SPEC = {
    feature: AI_GOAL_FEATURE,
    analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
    promptVersion: AI_GOAL_PROMPT_VERSION,
    system: AI_GOAL_SYSTEM_PROMPT,
    schemaName: 'goal_plan',
    schema: aiGoalPlanOutputSchema,
    maxOutputTokens: AI_GOAL_MAX_OUTPUT_TOKENS,
    effort: AI_GOAL_EFFORT,
} satisfies AiFeatureSpec<typeof aiGoalPlanOutputSchema>

export type AiGoalRow = { id: string; params: AiGoalParams; createdAt: number }

export type AiGoalServiceDb = {
    listActiveGoals: (userId: string) => Promise<AiGoalRow[]>
    findActiveGoal: (params: { userId: string; goalId: string }) => Promise<AiGoalRow | null>
    insertGoal: (row: AiGoalRow & { userId: string }) => Promise<void>
    /** Marks an active goal archived; false when the user has no such active goal. */
    archiveGoal: (params: { userId: string; goalId: string; archivedAt: number }) => Promise<boolean>
}

type AiGoalServiceDeps = TableFeatureDeps & {
    db: AiGoalServiceDb
    runner: Pick<AiFeatureRunner, 'run'>
    results: Pick<AiResultService, 'findLatest'>
    newId: (prefix: string) => string
    now: () => number
}

export type AiGoalFailure =
    | { reason: 'table_not_found' }
    | { reason: 'goal_not_found' }
    | { reason: 'goal_limit'; limit: number }
    | { reason: 'invalid_goal'; field: 'level' | 'lamp' | 'count' | 'md5' }

const compareRemaining = (left: ChartProgress, right: ChartProgress) =>
    right.lampId - left.lampId ||
    right.assistGrade - left.assistGrade ||
    (right.exRate ?? -1) - (left.exRate ?? -1) ||
    left.levelIndex - right.levelIndex ||
    left.title.localeCompare(right.title) ||
    left.md5.localeCompare(right.md5)

const scopeOf = (params: AiGoalParams, charts: readonly ChartProgress[]) =>
    params.kind === 'ex_rate_on_chart' ? charts.filter((chart) => chart.md5 === params.md5) : charts.filter((chart) => chart.level === params.level)

const labelOf = (params: AiGoalParams, table: ProgressTable, scope: readonly ChartProgress[]) => {
    if (params.kind === 'lamp_count_at_level') return `${table.symbol}${params.level} 에서 ${params.lamp} 이상 ${params.count}곡`
    if (params.kind === 'clear_all_level') return `${table.symbol}${params.level} 전부 클리어`
    return `${scope[0]?.title || params.md5} EX ${params.ex_rate}%`
}

/**
 * Where a goal stands (plan 5.7), from the player's best records: what it asks for, what is reached, the charts still short of it
 * (closest first) and how many charts of the goal got a new best in the last GOAL_PACE_DAYS days.
 */
export const computeGoalProgress = (params: AiGoalParams, table: ProgressTable, charts: readonly ChartProgress[], now: number) => {
    const scope = scopeOf(params, charts)
    const recentUpdates = scope.filter((chart) => chart.playedAt !== null && chart.playedAt >= now - GOAL_PACE_DAYS * DAY_MS).length
    const label = labelOf(params, table, scope)
    if (params.kind === 'ex_rate_on_chart') {
        const exRate = scope[0]?.exRate ?? null
        const done = exRate !== null && exRate >= params.ex_rate
        const progress = {
            label,
            target: params.ex_rate,
            achieved: exRate ?? 0,
            done,
            remaining: done ? 0 : 1,
            recent_updates: recentUpdates,
        } satisfies AiGoalProgress
        return { progress, remainingCharts: done ? [] : scope }
    }
    const targetLampId = params.kind === 'lamp_count_at_level' ? clearLampToId(params.lamp) : null
    const reaches = (chart: ChartProgress) => (targetLampId === null ? isClearLampId(chart.lampId) : chart.lampId >= targetLampId)
    const achieved = scope.filter(reaches).length
    const target = params.kind === 'lamp_count_at_level' ? params.count : scope.length
    const done = achieved >= target
    const progress = {
        label,
        target,
        achieved,
        done,
        remaining: Math.max(0, target - achieved),
        recent_updates: recentUpdates,
    } satisfies AiGoalProgress
    return { progress, remainingCharts: done ? [] : scope.filter((chart) => !reaches(chart)).toSorted(compareRemaining) }
}

/** Checks a new goal against its table: the level exists, the lamp is a clear, the count fits the level, the chart is in the table. */
export const validateGoal = (params: AiGoalParams, table: ProgressTable) => {
    if (params.kind === 'ex_rate_on_chart')
        return table.entries.some((entry) => entry.md5 === params.md5) ? null : ({ reason: 'invalid_goal', field: 'md5' } as const)
    const charts = new Set(table.entries.filter((entry) => entry.level === params.level).map((entry) => entry.md5))
    if (charts.size === 0) return { reason: 'invalid_goal', field: 'level' } as const
    if (params.kind === 'clear_all_level') return null
    if (!isClearLampId(clearLampToId(params.lamp))) return { reason: 'invalid_goal', field: 'lamp' } as const
    return params.count > charts.size ? ({ reason: 'invalid_goal', field: 'count' } as const) : null
}

/**
 * Keeps the plan inside the goal (plan 5.7): steps with a week outside 1 to AI_GOAL_LIMIT.MAX_WEEKS, an unknown `check_key` or a
 * focus that does not fit one line are dropped, of the steps that pass a week's first one is kept and its repeats are dropped (so
 * `week_index` is unique and the screen keys on it), chart md5s outside the remaining charts are dropped, and the steps go in week
 * order. With no step left the answer is rejected.
 */
export const refineGoalPlanOutput = (output: AiGoalPlanOutput, digest: { remaining: readonly { md5: string }[] }) => {
    const allowed = new Set(digest.remaining.map((chart) => chart.md5))
    const keys = collectDigestKeys(digest)
    const steps = output.steps
        .filter(
            (step) =>
                step.week_index >= 1 &&
                step.week_index <= AI_GOAL_LIMIT.MAX_WEEKS &&
                keys.has(step.check_key) &&
                fitsLine(step.focus, AI_TEXT_LIMIT.REASON_MAX_CHARS),
        )
        .filter((step, index, valid) => valid.findIndex((other) => other.week_index === step.week_index) === index)
        .map((step) => ({ ...step, focus: step.focus.trim(), chart_md5s: [...new Set(step.chart_md5s.filter((md5) => allowed.has(md5)))] }))
        .toSorted((left, right) => left.week_index - right.week_index)
        .slice(0, AI_GOAL_LIMIT.MAX_WEEKS)
    if (steps.length === 0) return null
    return { summary: cutText(output.summary, AI_TEXT_LIMIT.SUMMARY_MAX_CHARS), steps }
}

export const goalSubjectKey = (goalId: string) => `goal:${goalId}`

export const createAiGoalService = (deps: AiGoalServiceDeps) => {
    const progressOf = async (userId: string, goal: AiGoalRow) => {
        const table = deps.loadTable(goal.params.table)
        if (!table) return null
        const records = (await deps.loadRecords({ tableId: table.id, userId })) ?? {}
        const { charts } = readTableProgress(table, records)
        return { table, ...computeGoalProgress(goal.params, table, charts, deps.now()) }
    }

    const toGoalOutput = (goal: AiGoalRow, progress: AiGoalProgress | null) =>
        ({
            id: goal.id,
            params: goal.params,
            created_at: goal.createdAt,
            progress,
        }) satisfies AiGoalOutput

    const buildPlan = async (params: { userId: string; goalId: string }) => {
        const goal = await deps.db.findActiveGoal(params)
        if (!goal) return null
        const state = await progressOf(params.userId, goal)
        const shown = state?.remainingCharts.slice(0, GOAL_REMAINING_FOR_AI) ?? []
        const { shares: tags } = await deps.loadChartTags(shown.map((chart) => chart.md5))
        const remaining = shown.map((chart) => toCandidate(chart, tags))
        const progress = state?.progress ?? null
        return {
            goal,
            progress,
            allRemaining: state?.remainingCharts ?? [],
            remaining,
            digest: {
                goal: progress ? { key: 'goal', kind: goal.params.kind, ...progress } : null,
                pace: { key: 'pace', recent_updates: progress?.recent_updates ?? 0, days: GOAL_PACE_DAYS, weeks: GOAL_PACE_WEEKS },
                remaining,
                max_weeks: AI_GOAL_LIMIT.MAX_WEEKS,
            },
        }
    }

    const noticeOf = (built: NonNullable<Awaited<ReturnType<typeof buildPlan>>>) => {
        if (!built.progress) return { need: 'table' }
        if (built.progress.done) return { need: 'goal_reached' }
        if (built.remaining.length === 0) return { need: 'remaining_charts' }
        return null
    }

    return {
        list: async (userId: string) =>
            Promise.all(
                (await deps.db.listActiveGoals(userId)).map(async (goal) => toGoalOutput(goal, (await progressOf(userId, goal))?.progress ?? null)),
            ),

        create: async (params: { userId: string; input: AiGoalParams }) => {
            const table = deps.loadTable(params.input.table)
            if (!table) return { ok: false, reason: 'table_not_found' } as const
            const invalid = validateGoal(params.input, table)
            if (invalid) return { ok: false, ...invalid } as const
            const active = await deps.db.listActiveGoals(params.userId)
            if (active.length >= AI_GOAL_LIMIT.ACTIVE_MAX) return { ok: false, reason: 'goal_limit', limit: AI_GOAL_LIMIT.ACTIVE_MAX } as const
            const row = { id: deps.newId(AI_GOAL_ID_PREFIX), params: { ...params.input, table: table.id }, createdAt: deps.now() }
            await deps.db.insertGoal({ ...row, userId: params.userId })
            return { ok: true, value: toGoalOutput(row, (await progressOf(params.userId, row))?.progress ?? null) } as const
        },

        archive: (params: { userId: string; goalId: string }) => deps.db.archiveGoal({ ...params, archivedAt: deps.now() }),

        /** The goal's remaining charts as practice folder candidates (feature 6), closest to the goal first. */
        remainingCandidates: async (params: { userId: string; goalId: string; limit: number }) => {
            const built = await buildPlan(params)
            if (!built) return null
            const shown = built.allRemaining.slice(0, params.limit)
            const { shares: tags } = await deps.loadChartTags(shown.map((chart) => chart.md5))
            return {
                tableId: built.goal.params.table,
                label: built.progress?.label ?? '',
                candidates: shown.map((chart) => toCandidate(chart, tags)),
            }
        },

        planView: async (params: { userId: string; goalId: string }) => {
            const built = await buildPlan(params)
            if (!built) return null
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: AI_GOAL_SPEC,
                userId: params.userId,
                subjectKey: goalSubjectKey(built.goal.id),
                digest: built.digest,
            })
            return {
                goal: toGoalOutput(built.goal, built.progress),
                remaining: built.remaining,
                notice: noticeOf(built),
                ai,
            } satisfies AiGoalPlanView
        },

        /** Asks the user's AI for a week-by-week plan; a reached goal or one with no chart left makes no call. */
        interpret: async (params: { userId: string; goalId: string }) => {
            const built = await buildPlan(params)
            if (!built) return null
            const notice = noticeOf(built)
            if (notice) return notEnoughData(notice.need)
            return runAiFeature(deps.runner, {
                ...AI_GOAL_SPEC,
                userId: params.userId,
                subjectKey: goalSubjectKey(built.goal.id),
                digest: built.digest,
                refine: (output) => refineGoalPlanOutput(output, built.digest),
            })
        },
    }
}

export type AiGoalService = ReturnType<typeof createAiGoalService>
