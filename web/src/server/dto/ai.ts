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
