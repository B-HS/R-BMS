import {
    AI_GOAL_LIMIT,
    AI_TEXT_LIMIT,
    aiGoalPlanOutputSchema,
    type AiEffort,
    type AiGoalInput,
    type AiGoalOutput,
    type AiGoalParams,
    type AiGoalPlanOutput,
    type AiGoalPlanView,
    type AiGoalProgress,
} from '@server/dto/ai'
import type { WeaknessProfileView } from '@server/dto/ai-play'
import { clearLampToId } from '@server/dto/common'
import {
    AI_OUTPUT_LANGUAGE_NAME,
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
    GOAL_DENSITY_NPS_MARGIN,
    GOAL_PACE_DAYS,
    GOAL_PACE_WEEKS,
    GOAL_REMAINING_FOR_AI,
    SCORE_FEATURE_ANALYSIS_VERSION,
} from '@server/service/domain/ai/score-feature.constants'
import {
    isClearLampId,
    practicePoolOf,
    rankWeaknessCharts,
    readTableProgress,
    toCandidate,
    type ChartProgress,
    type ChartTagLoad,
    type ProgressTable,
} from '@server/service/domain/ai/table-progress'
import type { WeaknessProfileService } from '@server/service/domain/ai/weakness-profile.service'
import { roundRatio } from '@server/service/domain/analysis/chart-features'
import { parseWeaknessKey, WEAKNESS_KEY, type WeaknessKind } from '@server/service/domain/analysis/weakness-key'
import type { Locale } from '@shared/i18n/locale'

export const AI_GOAL_FEATURE = 'goal_plan'
export const AI_GOAL_PROMPT_VERSION = 3
export const AI_GOAL_MAX_OUTPUT_TOKENS = 8_000
export const AI_GOAL_EFFORT = 'low' satisfies AiEffort
export const AI_GOAL_ID_PREFIX = 'aig_'

const DAY_MS = 86_400_000

export const buildAiGoalSystemPrompt = (language: Locale) =>
    [
        'You plan the weeks a rhythm game (BMS) player needs to reach one goal they set on a difficulty table.',
        DIGEST_IS_DATA_RULE,
        '`goal` is the goal and where the player stands, `pace` how many charts of the goal had a new best in the last weeks, `remaining` the charts still short of the goal.',
        "For a `weakness_residual` or `density_limit` goal, `goal.current` is the value of the player's profile now, `goal.baseline` the value when the goal was made and `goal.target` the value to reach, and `remaining` are the charts that train that weakness or density most.",
        `Write at most ${AI_GOAL_LIMIT.MAX_WEEKS} steps, one per week, with \`week_index\` from 1 to \`max_weeks\` in order.`,
        `Each step has one ${AI_OUTPUT_LANGUAGE_NAME[language]} sentence \`focus\`, the \`md5\` values of the charts to play that week taken from \`remaining\` only (\`chart_md5s\`), and a \`check_key\` naming the digest item that tells whether the step worked.`,
        ASSIST_GRADE_RULE,
        DIGEST_KEY_RULE,
        `\`summary\` is two ${AI_OUTPUT_LANGUAGE_NAME[language]} sentences at most.`,
    ].join('\n')

export const aiGoalSpecOf = (language: Locale) =>
    ({
        feature: AI_GOAL_FEATURE,
        analysisVersion: SCORE_FEATURE_ANALYSIS_VERSION,
        promptVersion: AI_GOAL_PROMPT_VERSION,
        system: buildAiGoalSystemPrompt(language),
        schemaName: 'goal_plan',
        schema: aiGoalPlanOutputSchema,
        maxOutputTokens: AI_GOAL_MAX_OUTPUT_TOKENS,
        effort: AI_GOAL_EFFORT,
    }) satisfies AiFeatureSpec<typeof aiGoalPlanOutputSchema>

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
    /** The player's weakness profile, the only way to the `weakness_residual` and `density_limit` goals. */
    profile?: Pick<WeaknessProfileService, 'view'>
    newId: (prefix: string) => string
    now: () => number
}

export type AiGoalFailure =
    | { reason: 'table_not_found' }
    | { reason: 'goal_not_found' }
    | { reason: 'goal_limit'; limit: number }
    | { reason: 'invalid_goal'; field: 'level' | 'lamp' | 'count' | 'md5' | 'key' }
    | { reason: 'not_enough_data'; need: string }

type TableGoalParams = Extract<AiGoalParams, { kind: 'lamp_count_at_level' | 'clear_all_level' | 'ex_rate_on_chart' }>
type ProfileGoalParams = Extract<AiGoalParams, { kind: 'weakness_residual' | 'density_limit' }>

/** The weakness kinds whose value is an EX ratio difference a residual goal can aim at; timing (ms) and contexts (lift) are not. */
const RESIDUAL_GOAL_KINDS = ['tag', 'solo', 'combo', 'lane', 'hand', 'bpm', 'soflan', 'fatigue', 'density'] as const satisfies readonly WeaknessKind[]
const DENSITY_LIMIT_KEY = WEAKNESS_KEY.density('limit')

const isProfileGoal = (params: AiGoalParams): params is ProfileGoalParams => params.kind === 'weakness_residual' || params.kind === 'density_limit'

/** The weakness key a profile goal is about: its own key, or the density limit's. */
const profileGoalKeyOf = (params: ProfileGoalParams) => (params.kind === 'weakness_residual' ? params.key : DENSITY_LIMIT_KEY)

/**
 * The profile value a profile goal reads (plan 2026-11-04 2.6 feature 7): a weakness key's cell estimate `R_c`, or the EX density
 * limit, which is at least the densest NPS seen when the player never dropped below it. Null when the profile has no such value.
 */
export const profileGoalValueOf = (params: Pick<ProfileGoalParams, 'kind'> & { key?: string }, profile: WeaknessProfileView) => {
    if (params.kind === 'density_limit')
        return profile.density.limit_nps_ex ?? (profile.density.ceiling_reached_ex ? null : profile.density.max_seen_nps)
    return profile.cells.find((cell) => cell.key === params.key)?.value ?? null
}

const compareRemaining = (left: ChartProgress, right: ChartProgress) =>
    right.lampId - left.lampId ||
    right.assistGrade - left.assistGrade ||
    (right.exRate ?? -1) - (left.exRate ?? -1) ||
    left.levelIndex - right.levelIndex ||
    left.title.localeCompare(right.title) ||
    left.md5.localeCompare(right.md5)

const scopeOf = (params: TableGoalParams, charts: readonly ChartProgress[]) =>
    params.kind === 'ex_rate_on_chart' ? charts.filter((chart) => chart.md5 === params.md5) : charts.filter((chart) => chart.level === params.level)

const labelOf = (params: TableGoalParams, table: ProgressTable, scope: readonly ChartProgress[]) => {
    if (params.kind === 'lamp_count_at_level') return `${table.symbol}${params.level} 에서 ${params.lamp} 이상 ${params.count}곡`
    if (params.kind === 'clear_all_level') return `${table.symbol}${params.level} 전부 클리어`
    return `${scope[0]?.title || params.md5} EX ${params.ex_rate}%`
}

/**
 * Where a goal stands (plan 5.7), from the player's best records: what it asks for, what is reached, the charts still short of it
 * (closest first) and how many charts of the goal got a new best in the last GOAL_PACE_DAYS days.
 */
const recentUpdatesOf = (charts: readonly ChartProgress[], now: number) =>
    charts.filter((chart) => chart.playedAt !== null && chart.playedAt >= now - GOAL_PACE_DAYS * DAY_MS).length

export const computeGoalProgress = (params: TableGoalParams, table: ProgressTable, charts: readonly ChartProgress[], now: number) => {
    const scope = scopeOf(params, charts)
    const recentUpdates = recentUpdatesOf(scope, now)
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

/**
 * Where a profile goal stands (plan 2026-11-04 2.6 feature 7): the profile value now against the target, reached when it is at the
 * target or above, and `ratio` the share of the way from the baseline to the target (1 once reached, null without a baseline or a
 * value). The plan charts are the ones that train the weakness or density, and `recent_updates` counts those with a new best in
 * the last GOAL_PACE_DAYS days.
 */
export const computeProfileGoalProgress = (params: {
    goal: ProfileGoalParams
    current: number | null
    charts: readonly ChartProgress[]
    now: number
}) => {
    const { goal, current } = params
    const target = goal.kind === 'weakness_residual' ? goal.target_residual : goal.target_nps
    const done = current !== null && current >= target
    const span = goal.baseline === null ? 0 : target - goal.baseline
    const ratioOf = () => {
        if (done) return 1
        if (current === null || goal.baseline === null || span <= 0) return null
        return roundRatio(Math.min(1, Math.max(0, (current - goal.baseline) / span)))
    }
    const label = goal.kind === 'weakness_residual' ? `${goal.key} 잔차 ${target} 이상` : `밀도 한계 ${target} NPS 이상`
    const progress = {
        label,
        target,
        achieved: current ?? goal.baseline ?? 0,
        done,
        remaining: done ? 0 : 1,
        recent_updates: recentUpdatesOf(params.charts, params.now),
        current,
        ratio: ratioOf(),
    } satisfies AiGoalProgress
    return { progress, remainingCharts: done ? [] : params.charts }
}

/** The charts a profile goal plans with: a weakness's best-training pool charts, or the pool charts nearest the target NPS. */
export const profileGoalChartsOf = (params: {
    goal: ProfileGoalParams
    pool: readonly ChartProgress[]
    load: ChartTagLoad
    limitNps: number | null
    limit: number
}) => {
    const { goal } = params
    if (goal.kind === 'weakness_residual')
        return rankWeaknessCharts({ pool: params.pool, key: goal.key, load: params.load, limitNps: params.limitNps, limit: params.limit }).map(
            ({ chart }) => chart,
        )
    return params.pool
        .flatMap((chart) => {
            const p90 = params.load.extras?.get(chart.md5)?.nps.p90_2s
            if (p90 === undefined || Math.abs(p90 - goal.target_nps) > GOAL_DENSITY_NPS_MARGIN) return []
            return [{ chart, distance: Math.abs(p90 - goal.target_nps) }]
        })
        .toSorted(
            (left, right) =>
                left.distance - right.distance || left.chart.levelIndex - right.chart.levelIndex || left.chart.md5.localeCompare(right.chart.md5),
        )
        .slice(0, params.limit)
        .map(({ chart }) => chart)
}

/**
 * Checks a new profile goal against the profile: a residual goal's key must be one of the weaknesses (else not enough data,
 * `weakness_profile`) of a kind whose value is an EX ratio difference; a density goal needs a density value. Returns the baseline.
 */
export const validateProfileGoal = (
    params: Extract<AiGoalInput, { kind: 'weakness_residual' | 'density_limit' }>,
    profile: WeaknessProfileView | null,
) => {
    if (!profile) return { ok: false, reason: 'not_enough_data', need: 'weakness_profile' } as const
    if (params.kind === 'weakness_residual') {
        if (!profile.weaknesses.some((item) => item.key === params.key))
            return { ok: false, reason: 'not_enough_data', need: 'weakness_profile' } as const
        const parsed = parseWeaknessKey(params.key)
        const isResidual = parsed !== null && params.key !== DENSITY_LIMIT_KEY && RESIDUAL_GOAL_KINDS.some((kind) => kind === parsed.kind)
        if (!isResidual) return { ok: false, reason: 'invalid_goal', field: 'key' } as const
    }
    const baseline = profileGoalValueOf(params, profile)
    if (params.kind === 'density_limit' && baseline === null) return { ok: false, reason: 'not_enough_data', need: 'weakness_profile' } as const
    return { ok: true, baseline } as const
}

/** Checks a new goal against its table: the level exists, the lamp is a clear, the count fits the level, the chart is in the table. */
export const validateGoal = (params: TableGoalParams, table: ProgressTable) => {
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
    /** The profile read once per request, only when a profile goal asks for it. */
    const profileReader = (userId: string) => {
        let pending: Promise<WeaknessProfileView | null> | null = null
        return () => {
            pending ??= deps.profile ? deps.profile.view({ userId }) : Promise.resolve(null)
            return pending
        }
    }

    const progressOf = async (userId: string, goal: AiGoalRow, readProfile = profileReader(userId)) => {
        const table = deps.loadTable(goal.params.table)
        if (!table) return null
        const records = (await deps.loadRecords({ tableId: table.id, userId })) ?? {}
        const progress = readTableProgress(table, records)
        const params = goal.params
        if (!isProfileGoal(params)) return { table, ...computeGoalProgress(params, table, progress.charts, deps.now()) }
        const profile = await readProfile()
        const pool = practicePoolOf(progress)
        const load = await deps.loadChartTags(pool.map((chart) => chart.md5))
        const charts = profileGoalChartsOf({ goal: params, pool, load, limitNps: profile?.density.limit_nps_ex ?? null, limit: pool.length })
        const current = profile ? profileGoalValueOf(params, profile) : null
        return { table, ...computeProfileGoalProgress({ goal: params, current, charts, now: deps.now() }) }
    }

    /** A new goal checked and given what is kept with it: a table goal against its table, a profile goal against the profile (with its baseline). */
    const checkNewGoal = async (input: AiGoalInput, table: ProgressTable, readProfile: () => Promise<WeaknessProfileView | null>) => {
        if (input.kind === 'weakness_residual' || input.kind === 'density_limit') {
            const checked = validateProfileGoal(input, await readProfile())
            return checked.ok ? ({ ok: true, params: { ...input, table: table.id, baseline: checked.baseline } } as const) : checked
        }
        const invalid = validateGoal(input, table)
        return invalid ? ({ ok: false, ...invalid } as const) : ({ ok: true, params: { ...input, table: table.id } } as const)
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
                goal: progress
                    ? {
                          key: 'goal',
                          kind: goal.params.kind,
                          ...progress,
                          ...(isProfileGoal(goal.params) ? { focus_key: profileGoalKeyOf(goal.params), baseline: goal.params.baseline } : {}),
                      }
                    : null,
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
        list: async (userId: string) => {
            const readProfile = profileReader(userId)
            return Promise.all(
                (await deps.db.listActiveGoals(userId)).map(async (goal) =>
                    toGoalOutput(goal, (await progressOf(userId, goal, readProfile))?.progress ?? null),
                ),
            )
        },

        /**
         * Adds a goal. A table goal is checked against the table; a profile goal against the profile, which also gives the baseline
         * kept with it (a key not among the weaknesses, or no profile, is not enough data, before anything is stored).
         */
        create: async (params: { userId: string; input: AiGoalInput }) => {
            const { input } = params
            const table = deps.loadTable(input.table)
            if (!table) return { ok: false, reason: 'table_not_found' } as const
            const readProfile = profileReader(params.userId)
            const checked = await checkNewGoal(input, table, readProfile)
            if (!checked.ok) return checked
            const active = await deps.db.listActiveGoals(params.userId)
            if (active.length >= AI_GOAL_LIMIT.ACTIVE_MAX) return { ok: false, reason: 'goal_limit', limit: AI_GOAL_LIMIT.ACTIVE_MAX } as const
            const row = { id: deps.newId(AI_GOAL_ID_PREFIX), params: checked.params, createdAt: deps.now() } satisfies AiGoalRow
            await deps.db.insertGoal({ ...row, userId: params.userId })
            return { ok: true, value: toGoalOutput(row, (await progressOf(params.userId, row, readProfile))?.progress ?? null) } as const
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

        planView: async (params: { userId: string; goalId: string; language: Locale }) => {
            const built = await buildPlan(params)
            if (!built) return null
            const ai = await readStoredInterpretation({
                results: deps.results,
                spec: aiGoalSpecOf(params.language),
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
        interpret: async (params: { userId: string; goalId: string; language: Locale }) => {
            const built = await buildPlan(params)
            if (!built) return null
            const notice = noticeOf(built)
            if (notice) return notEnoughData(notice.need)
            return runAiFeature(deps.runner, {
                ...aiGoalSpecOf(params.language),
                userId: params.userId,
                subjectKey: goalSubjectKey(built.goal.id),
                digest: built.digest,
                refine: (output) => refineGoalPlanOutput(output, built.digest),
            })
        },
    }
}

export type AiGoalService = ReturnType<typeof createAiGoalService>
