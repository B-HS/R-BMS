import { z } from 'zod'
import { clearLampSchema, gaugeTypeSchema } from '@server/dto/common'

export const AI_PROVIDER_IDS = ['anthropic', 'openai', 'ollama'] as const
export const aiProviderIdSchema = z.enum(AI_PROVIDER_IDS)
export type AiProviderId = z.infer<typeof aiProviderIdSchema>

export const AI_FAILURE_KIND = {
    INVALID_KEY: 'invalid_key',
    FORBIDDEN: 'forbidden',
    MODEL_UNAVAILABLE: 'model_unavailable',
    QUOTA: 'quota',
    RATE_LIMITED: 'rate_limited',
    UNAVAILABLE: 'unavailable',
    REFUSED: 'refused',
    TRUNCATED: 'truncated',
    INVALID_OUTPUT: 'invalid_output',
    BAD_REQUEST: 'bad_request',
} as const
export type AiFailureKind = (typeof AI_FAILURE_KIND)[keyof typeof AI_FAILURE_KIND]

export const AI_EFFORT_LEVELS = ['low', 'medium', 'high', 'xhigh', 'max'] as const
export const aiEffortSchema = z.enum(AI_EFFORT_LEVELS)
export type AiEffort = z.infer<typeof aiEffortSchema>

/**
 * Normalized token usage. `inputTokens` always includes the cached part, so the
 * three providers can be summed against the same monthly cap.
 */
export const aiUsageSchema = z.object({
    inputTokens: z.number().int().nonnegative(),
    outputTokens: z.number().int().nonnegative(),
    cachedInputTokens: z.number().int().nonnegative(),
})
export type AiUsage = z.infer<typeof aiUsageSchema>

export const AI_USAGE_LIMIT_REASONS = ['daily_calls', 'monthly_tokens', 'concurrent_runs'] as const

export const AI_LIMIT = {
    API_KEY_MIN_LENGTH: 8,
    API_KEY_MAX_LENGTH: 512,
    KEY_HINT_LENGTH: 4,
    MODEL_MAX_LENGTH: 128,
    WRITE_MAX_BYTES: 4 * 1024,
    DAILY_CALLS_MAX: 1000,
    CONCURRENT_RUNS_MAX: 5,
    MONTHLY_TOKEN_LIMIT_MAX: 1_000_000_000_000,
} as const

export const AI_DEFAULT_LIMITS = {
    DAILY_CALLS: 30,
    CONCURRENT_RUNS: 1,
    DEFAULT_MONTHLY_TOKEN_LIMIT: null,
} as const

const PRINTABLE_WITHOUT_SPACE = /^[\x21-\x7e]+$/
const MODEL_ID_PATTERN = /^[A-Za-z0-9._:/@-]+$/

export const aiFailureKindSchema = z.enum(AI_FAILURE_KIND)
export const aiModelIdSchema = z.string().min(1).max(AI_LIMIT.MODEL_MAX_LENGTH).regex(MODEL_ID_PATTERN)

export type AiUsageLimitReason = (typeof AI_USAGE_LIMIT_REASONS)[number]

export const aiProviderParamSchema = z.object({ provider: aiProviderIdSchema })

export const aiCredentialPutSchema = z.object({
    api_key: z
        .string()
        .transform((value) => value.trim())
        .refine(
            (value) =>
                value.length >= AI_LIMIT.API_KEY_MIN_LENGTH && value.length <= AI_LIMIT.API_KEY_MAX_LENGTH && PRINTABLE_WITHOUT_SPACE.test(value),
            { message: `api_key must be ${AI_LIMIT.API_KEY_MIN_LENGTH} to ${AI_LIMIT.API_KEY_MAX_LENGTH} printable characters without spaces` },
        ),
})

export const aiCredentialModelSchema = z.object({ model: aiModelIdSchema })

const monthlyTokenLimitSchema = z.number().int().positive().max(AI_LIMIT.MONTHLY_TOKEN_LIMIT_MAX).nullable()

export const aiUserSettingPutSchema = z.object({
    monthly_token_limit: monthlyTokenLimitSchema,
    default_provider: aiProviderIdSchema.nullable(),
})

export const aiOperatorLimitsPutSchema = z.object({
    daily_calls: z.number().int().positive().max(AI_LIMIT.DAILY_CALLS_MAX),
    concurrent_runs: z.number().int().positive().max(AI_LIMIT.CONCURRENT_RUNS_MAX),
    default_monthly_token_limit: monthlyTokenLimitSchema,
})

export type AiCredentialPutInput = z.infer<typeof aiCredentialPutSchema>
export type AiUserSettingPutInput = z.infer<typeof aiUserSettingPutSchema>
export type AiOperatorLimitsPutInput = z.infer<typeof aiOperatorLimitsPutSchema>

export const aiCredentialSchema = z.object({
    provider: aiProviderIdSchema,
    key_hint: z.string(),
    model: z.string().nullable(),
    created_at: z.number().int(),
    last_validated_at: z.number().int().nullable(),
    last_error_kind: aiFailureKindSchema.nullable(),
})

export const aiModelSchema = z.object({
    id: z.string(),
    label: z.string(),
    structured: z.boolean().nullable(),
})

export const aiTokenTotalsSchema = z.object({
    calls: z.number().int(),
    input_tokens: z.number().int(),
    output_tokens: z.number().int(),
    cached_input_tokens: z.number().int(),
})

export const aiOperatorLimitsSchema = aiOperatorLimitsPutSchema.extend({
    updated_at: z.number().int().nullable(),
})

export const aiUserSettingSchema = aiUserSettingPutSchema.extend({
    updated_at: z.number().int().nullable(),
})

export const aiUsageSummarySchema = z.object({
    day: z.string(),
    month: z.string(),
    today: aiTokenTotalsSchema,
    this_month: aiTokenTotalsSchema,
    limits: z.object({
        daily_calls: z.number().int(),
        concurrent_runs: z.number().int(),
        monthly_token_limit: z.number().int().nullable(),
    }),
})

export type AiCredentialOutput = z.infer<typeof aiCredentialSchema>
export type AiModelOutput = z.infer<typeof aiModelSchema>
export type AiTokenTotalsOutput = z.infer<typeof aiTokenTotalsSchema>
export type AiOperatorLimitsOutput = z.infer<typeof aiOperatorLimitsSchema>
export type AiUserSettingOutput = z.infer<typeof aiUserSettingSchema>
export type AiUsageSummaryOutput = z.infer<typeof aiUsageSummarySchema>

export const AI_RECORD_QUERY_FIELDS = ['level', 'lamp', 'ex_rate', 'bp', 'played_at', 'random', 'gauge', 'scratch_auto', 'title', 'table'] as const
export const AI_RECORD_QUERY_OPS = ['eq', 'ne', 'gte', 'lte', 'in', 'contains'] as const
export const AI_RECORD_QUERY_SORT_FIELDS = ['played_at', 'ex_rate', 'bp', 'level', 'lamp'] as const
export const AI_RECORD_QUERY_SORT_DIRS = ['asc', 'desc'] as const
export const AI_RECORD_QUERY_SOURCES = ['scores', 'table'] as const
export const AI_RECORD_QUERY_ISSUE_REASONS = ['op_not_allowed', 'value_invalid', 'too_many_filters'] as const

export type AiRecordQueryField = (typeof AI_RECORD_QUERY_FIELDS)[number]
export type AiRecordQueryOp = (typeof AI_RECORD_QUERY_OPS)[number]

export const AI_RECORD_QUERY_LIMIT = {
    QUESTION_MAX_CHARS: 200,
    MAX_FILTERS: 8,
    MAX_IN_VALUES: 20,
    VALUE_MAX_CHARS: 200,
    TITLE_MAX_CHARS: 100,
    ROWS_MIN: 1,
    ROWS_MAX: 100,
    EX_RATE_MAX_PERCENT: 100,
    EX_RATE_DECIMALS: 2,
    EXPLANATION_MAX_CHARS: 300,
    WRITE_MAX_BYTES: 8 * 1024,
} as const

/** Which comparisons each field takes. A filter outside this table is reported back and the query does not run. */
export const AI_RECORD_QUERY_ALLOWED_OPS = {
    level: ['eq', 'ne', 'in', 'gte', 'lte'],
    lamp: ['eq', 'ne', 'in', 'gte', 'lte'],
    ex_rate: ['gte', 'lte'],
    bp: ['eq', 'gte', 'lte'],
    played_at: ['gte', 'lte'],
    random: ['eq', 'ne', 'in'],
    gauge: ['eq', 'ne', 'in'],
    scratch_auto: ['eq'],
    title: ['contains'],
    table: ['eq'],
} as const satisfies Record<AiRecordQueryField, readonly AiRecordQueryOp[]>

export const aiRecordQueryFilterSchema = z.object({
    field: z.enum(AI_RECORD_QUERY_FIELDS),
    op: z.enum(AI_RECORD_QUERY_OPS),
    value: z.string(),
})

/**
 * What the model returns for a record question. Every field is required and there are no unions, so the same schema works for
 * every provider; the value is a string the server parses per field, and the server clamps `limit` instead of rejecting it.
 */
export const aiRecordQueryOutputSchema = z.object({
    filters: z.array(aiRecordQueryFilterSchema),
    sort_field: z.enum(AI_RECORD_QUERY_SORT_FIELDS),
    sort_dir: z.enum(AI_RECORD_QUERY_SORT_DIRS),
    limit: z.number().int(),
    explanation: z.string(),
})

export const aiRecordQueryAskSchema = z.object({
    question: z.string().trim().min(1).max(AI_RECORD_QUERY_LIMIT.QUESTION_MAX_CHARS),
})

export const aiRecordQueryRunSchema = aiRecordQueryOutputSchema.omit({ explanation: true })

export type AiRecordQueryFilter = z.infer<typeof aiRecordQueryFilterSchema>
export type AiRecordQueryOutput = z.infer<typeof aiRecordQueryOutputSchema>
export type AiRecordQueryRunInput = z.infer<typeof aiRecordQueryRunSchema>

export const aiRecordQueryIssueSchema = z.object({
    index: z.number().int(),
    reason: z.enum(AI_RECORD_QUERY_ISSUE_REASONS),
})

export const aiRecordQueryRowSchema = z.object({
    score_id: z.string().nullable(),
    chart_sha256: z.string().nullable(),
    chart_md5: z.string().nullable(),
    title: z.string(),
    artist: z.string(),
    table_level: z.string().nullable(),
    lamp: clearLampSchema,
    ex_score: z.number().int().nullable(),
    ex_rate: z.number().nullable(),
    bp: z.number().int().nullable(),
    played_at: z.number().int().nullable(),
    random: z.string().nullable(),
    gauge: gaugeTypeSchema.nullable(),
    scratch_auto: z.boolean().nullable(),
})

export const aiRecordQueryAnswerSchema = z.object({
    plan: aiRecordQueryRunSchema.extend({ explanation: z.string().nullable() }),
    issues: z.array(aiRecordQueryIssueSchema),
    result: z.object({ source: z.enum(AI_RECORD_QUERY_SOURCES), rows: z.array(aiRecordQueryRowSchema) }).nullable(),
    ai: z
        .object({
            cached: z.boolean(),
            provider: aiProviderIdSchema,
            model: z.string(),
            usage: aiUsageSchema,
            created_at: z.number().int(),
        })
        .nullable(),
})

export type AiRecordQueryIssue = z.infer<typeof aiRecordQueryIssueSchema>
export type AiRecordQueryRowOutput = z.infer<typeof aiRecordQueryRowSchema>
export type AiRecordQueryAnswerOutput = z.infer<typeof aiRecordQueryAnswerSchema>

export const AI_INSIGHT_TONES = ['strength', 'weakness', 'neutral'] as const

/** Text lengths the server keeps in AI output. A longer summary is cut; a longer item is dropped, never cut (plan 5.6). */
export const AI_TEXT_LIMIT = {
    SUMMARY_MAX_CHARS: 500,
    REASON_MAX_CHARS: 300,
    INSIGHT_TITLE_MAX_CHARS: 80,
    INSIGHT_BODY_MAX_CHARS: 400,
    KEY_MAX_CHARS: 128,
} as const

/** One claim of an AI answer; `key` points at the digest item it rests on, and a key that is not in the digest drops the claim. */
export const aiInsightSchema = z.object({
    key: z.string(),
    title: z.string(),
    body: z.string(),
    tone: z.enum(AI_INSIGHT_TONES),
})

export type AiInsight = z.infer<typeof aiInsightSchema>

/** A stored AI answer as a screen shows it: `is_current` is false once the digest, prompt or model it was made from changed. */
export const aiInterpretationSchema = <TOutput extends z.ZodType>(output: TOutput) =>
    z.object({
        output,
        cached: z.boolean(),
        is_current: z.boolean(),
        provider: aiProviderIdSchema,
        model: z.string(),
        usage: aiUsageSchema,
        created_at: z.number().int(),
    })

export const aiDataNoticeSchema = z.object({ need: z.string() }).nullable()

const TABLE_ID_PATTERN = /^[a-z0-9][a-z0-9-]{0,63}$/
const MD5_PATTERN = /^[0-9a-f]{32}$/
const TAG_PATTERN = /^[a-z0-9_]{1,32}$/
const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/
const AI_GOAL_ID_PATTERN = /^aig_[0-9a-f]{30}$/

const aiTableIdSchema = z.string().regex(TABLE_ID_PATTERN)
const aiMd5Schema = z
    .string()
    .transform((value) => value.trim().toLowerCase())
    .refine((value) => MD5_PATTERN.test(value))

export const aiGoalIdSchema = z.string().regex(AI_GOAL_ID_PATTERN)

export const aiTableQuerySchema = z.object({ table: aiTableIdSchema.optional() })

export const aiChartCandidateSchema = z.object({
    key: z.string(),
    md5: z.string(),
    title: z.string(),
    artist: z.string(),
    level: z.string(),
    lamp: clearLampSchema,
    ex_rate: z.number().nullable(),
    bp: z.number().int().nullable(),
    tags: z.array(z.object({ tag: z.string(), ratio: z.number() })),
})

export const AI_RECOMMEND_KINDS = ['failed', 'unplayed', 'lamp_up'] as const

export const aiLevelProgressSchema = z.object({
    key: z.string(),
    level: z.string(),
    charts: z.number().int(),
    played: z.number().int(),
    cleared: z.number().int(),
    hard_or_better: z.number().int(),
    clear_rate: z.number(),
})

export const aiRecommendOutputSchema = z.object({
    summary: z.string(),
    picks: z.array(z.object({ md5: z.string(), reason: z.string(), focus_key: z.string() })),
})

export const aiRecommendViewSchema = z.object({
    table: z.object({ id: z.string(), name: z.string(), symbol: z.string() }),
    frontier: aiLevelProgressSchema.nullable(),
    levels: z.array(aiLevelProgressSchema),
    candidates: z.array(aiChartCandidateSchema.extend({ kind: z.enum(AI_RECOMMEND_KINDS), score: z.number() })),
    pattern_coverage: z.object({ with_features: z.number().int(), candidates: z.number().int(), stale_features: z.number().int() }),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiRecommendOutputSchema).nullable(),
})

export type AiRecommendOutput = z.infer<typeof aiRecommendOutputSchema>
export type AiRecommendView = z.infer<typeof aiRecommendViewSchema>
export type AiChartCandidate = z.infer<typeof aiChartCandidateSchema>
export type AiLevelProgress = z.infer<typeof aiLevelProgressSchema>

export const AI_PRACTICE_SOURCES = ['recommend', 'tag', 'goal'] as const

export const AI_PRACTICE_LIMIT = {
    ITEMS_MAX: 30,
    NAME_PREFIX: 'AI 연습 · ',
    NAME_SUFFIX_MAX: 40,
    MEMO_MAX: 100,
    DESCRIPTION_MAX: 255,
    WRITE_MAX_BYTES: 32 * 1024,
} as const

export const aiPracticeSourceSchema = z.object({
    source: z.enum(AI_PRACTICE_SOURCES),
    table: aiTableIdSchema.optional(),
    tag: z.string().regex(TAG_PATTERN).optional(),
    goal_id: aiGoalIdSchema.optional(),
})

export const aiPracticeFolderOutputSchema = z.object({
    name_suffix: z.string(),
    description: z.string(),
    items: z.array(z.object({ md5: z.string(), memo: z.string() })),
})

export const aiPracticeFolderCreateSchema = aiPracticeSourceSchema.extend({
    name_suffix: z.string().trim().min(1).max(AI_PRACTICE_LIMIT.NAME_SUFFIX_MAX),
    description: z.string().trim().max(AI_PRACTICE_LIMIT.DESCRIPTION_MAX),
    items: z
        .array(z.object({ md5: aiMd5Schema, memo: z.string().trim().max(AI_PRACTICE_LIMIT.MEMO_MAX) }))
        .min(1)
        .max(AI_PRACTICE_LIMIT.ITEMS_MAX),
})

export const aiPracticeFolderViewSchema = z.object({
    source: z.enum(AI_PRACTICE_SOURCES),
    table_id: z.string(),
    tag: z.string().nullable(),
    goal_id: z.string().nullable(),
    candidates: z.array(aiChartCandidateSchema),
    folders: z.object({ used: z.number().int(), limit: z.number().int(), can_create: z.boolean() }),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiPracticeFolderOutputSchema).nullable(),
})

export const aiPracticeFolderResultSchema = z.object({
    outcome: z.enum(['created', 'updated']),
    folder_id: z.string(),
    name: z.string(),
    item_count: z.number().int(),
})

export type AiPracticeSourceInput = z.infer<typeof aiPracticeSourceSchema>
export type AiPracticeFolderOutput = z.infer<typeof aiPracticeFolderOutputSchema>
export type AiPracticeFolderCreateInput = z.infer<typeof aiPracticeFolderCreateSchema>
export type AiPracticeFolderView = z.infer<typeof aiPracticeFolderViewSchema>
export type AiPracticeFolderResult = z.infer<typeof aiPracticeFolderResultSchema>

export const AI_GOAL_KINDS = ['lamp_count_at_level', 'clear_all_level', 'ex_rate_on_chart'] as const
export type AiGoalKind = (typeof AI_GOAL_KINDS)[number]

export const AI_GOAL_LIMIT = {
    ACTIVE_MAX: 10,
    LEVEL_MAX_CHARS: 32,
    COUNT_MAX: 2000,
    EX_RATE_MAX_PERCENT: 100,
    MAX_WEEKS: 8,
    WRITE_MAX_BYTES: 4 * 1024,
} as const

const goalLevelSchema = z.string().trim().min(1).max(AI_GOAL_LIMIT.LEVEL_MAX_CHARS)

export const aiGoalParamsSchema = z.discriminatedUnion('kind', [
    z.object({
        kind: z.literal('lamp_count_at_level'),
        table: aiTableIdSchema,
        level: goalLevelSchema,
        lamp: clearLampSchema,
        count: z.number().int().min(1).max(AI_GOAL_LIMIT.COUNT_MAX),
    }),
    z.object({ kind: z.literal('clear_all_level'), table: aiTableIdSchema, level: goalLevelSchema }),
    z.object({
        kind: z.literal('ex_rate_on_chart'),
        table: aiTableIdSchema,
        md5: aiMd5Schema,
        ex_rate: z.number().positive().max(AI_GOAL_LIMIT.EX_RATE_MAX_PERCENT),
    }),
])

export type AiGoalParams = z.infer<typeof aiGoalParamsSchema>

export const aiGoalProgressSchema = z.object({
    label: z.string(),
    target: z.number(),
    achieved: z.number(),
    done: z.boolean(),
    remaining: z.number().int(),
    recent_updates: z.number().int(),
})

export const aiGoalSchema = z.object({
    id: z.string(),
    params: aiGoalParamsSchema,
    created_at: z.number().int(),
    progress: aiGoalProgressSchema.nullable(),
})

export const aiGoalPlanOutputSchema = z.object({
    summary: z.string(),
    steps: z.array(z.object({ week_index: z.number().int(), focus: z.string(), chart_md5s: z.array(z.string()), check_key: z.string() })),
})

export const aiGoalPlanViewSchema = z.object({
    goal: aiGoalSchema,
    remaining: z.array(aiChartCandidateSchema),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiGoalPlanOutputSchema).nullable(),
})

export type AiGoalOutput = z.infer<typeof aiGoalSchema>
export type AiGoalProgress = z.infer<typeof aiGoalProgressSchema>
export type AiGoalPlanOutput = z.infer<typeof aiGoalPlanOutputSchema>
export type AiGoalPlanView = z.infer<typeof aiGoalPlanViewSchema>

export const aiWeekQuerySchema = z.object({ week: z.string().regex(ISO_DATE_PATTERN).optional() })

export const aiWeeklyOutputSchema = z.object({
    headline: z.string(),
    highlights: z.array(aiInsightSchema),
    next_week: z.string(),
})

const weeklyChartChangeSchema = z.object({
    key: z.string(),
    title: z.string(),
    level: z.string().nullable(),
    from_lamp: clearLampSchema,
    to_lamp: clearLampSchema,
    from_ex_rate: z.number().nullable(),
    to_ex_rate: z.number().nullable(),
    scratch_auto: z.boolean(),
})

const weeklyTotalsSchema = z.object({
    plays: z.number().int(),
    charts: z.number().int(),
    active_days: z.number().int(),
    first_plays: z.number().int(),
    lamp_ups: z.number().int(),
    new_clears: z.number().int(),
    ex_ups: z.number().int(),
    fast: z.number().int(),
    slow: z.number().int(),
    fast_share: z.number().nullable(),
    mean_offset_ms: z.number().nullable(),
})

export const aiWeeklyViewSchema = z.object({
    week: z.object({ start: z.string(), end: z.string(), in_progress: z.boolean() }),
    totals: weeklyTotalsSchema,
    previous: weeklyTotalsSchema,
    lamp_changes: z.array(weeklyChartChangeSchema),
    ex_changes: z.array(weeklyChartChangeSchema),
    levels: z.array(z.object({ key: z.string(), level: z.string(), lamp_ups: z.number().int(), plays: z.number().int() })),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiWeeklyOutputSchema).nullable(),
})

export type AiWeeklyOutput = z.infer<typeof aiWeeklyOutputSchema>
export type AiWeeklyView = z.infer<typeof aiWeeklyViewSchema>

export const AI_OFFSET_DIRECTIONS = ['earlier', 'later', 'keep'] as const
export const AI_GREEN_DIRECTIONS = ['faster', 'slower', 'keep'] as const
export const AI_EVIDENCE_GRADES = ['strong', 'uncertain', 'insufficient'] as const

export const aiJudgeHabitOutputSchema = z.object({
    summary: z.string(),
    offset_direction: z.enum(AI_OFFSET_DIRECTIONS),
    offset_reasoning: z.string(),
    green_direction: z.enum(AI_GREEN_DIRECTIONS),
    green_reasoning: z.string(),
    notes: z.array(aiInsightSchema),
})

const greenEdgeSchema = z.object({ green: z.number().int(), lane_ms: z.number().int() })
const greenNumberSchema = greenEdgeSchema.extend({ is_estimated: z.boolean() })

export const aiJudgeHabitViewSchema = z.object({
    plays: z.number().int(),
    from: z.number().int().nullable(),
    to: z.number().int().nullable(),
    fast_slow: z.object({
        fast: z.number().int(),
        slow: z.number().int(),
        fast_share: z.number().nullable(),
        grade: z.enum(AI_EVIDENCE_GRADES),
        leaning: z.enum(['fast', 'slow']).nullable(),
        by_grade: z.array(z.object({ grade: z.enum(['great', 'good', 'bad']), early: z.number().int(), late: z.number().int() })),
    }),
    offset: z.object({
        plays: z.number().int(),
        notes: z.number().int(),
        mean_offset_ms: z.number().nullable(),
        grade: z.enum(AI_EVIDENCE_GRADES),
        direction: z.enum(AI_OFFSET_DIRECTIONS),
        suggested_shift_ms: z.number().int().nullable(),
        auto_adjust_plays: z.number().int(),
    }),
    avgjudge: z.object({
        plays: z.number().int(),
        older_ms: z.number().nullable(),
        newer_ms: z.number().nullable(),
        change_ms: z.number().nullable(),
        grade: z.enum(AI_EVIDENCE_GRADES),
    }),
    green: z.object({
        plays: z.number().int(),
        latest: greenNumberSchema.nullable(),
        median: greenNumberSchema.nullable(),
        constant_plays: z.number().int(),
        buckets: z.array(
            z.object({
                key: z.string(),
                from: greenEdgeSchema,
                to: greenEdgeSchema,
                plays: z.number().int(),
                fast_share: z.number().nullable(),
                ex_rate: z.number().nullable(),
            }),
        ),
    }),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiJudgeHabitOutputSchema).nullable(),
})

export type AiJudgeHabitOutput = z.infer<typeof aiJudgeHabitOutputSchema>
export type AiJudgeHabitView = z.infer<typeof aiJudgeHabitViewSchema>

/** The timing fields a score row's `extra` may carry (ADR 2026-10-15 avgjudge); anything else in it is ignored. */
export const aiScoreTimingExtraSchema = z.object({
    mean_offset_us: z.number().optional(),
    rule_version: z.number().optional(),
})
