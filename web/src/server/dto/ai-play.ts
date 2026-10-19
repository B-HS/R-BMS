import { z } from 'zod'
import { AI_EVIDENCE_GRADES, aiDataNoticeSchema, aiInsightSchema, aiInterpretationSchema, aiLevelProgressSchema } from '@server/dto/ai'
import { GAUGE_TYPES, RANDOM_OPTIONS, clearLampSchema } from '@server/dto/common'

/** The nine gauges as AI output names them, in the order of `GAUGE_TYPES` (`GaugeIndex::ALL`, rbms-judge). */
export const AI_GAUGE_NAMES = ['assist_easy', 'easy', 'normal', 'hard', 'ex_hard', 'hazard', 'class', 'ex_class', 'ex_hard_class'] as const
/** The lane options as AI output names them, in the order of `RANDOM_OPTIONS`. */
export const AI_OPTION_NAMES = ['off', 'mirror', 'random', 'r_random', 's_random', 'spiral', 'h_random', 'all_scratch', 'converge'] as const

export type AiGaugeName = (typeof AI_GAUGE_NAMES)[number]
export type AiOptionName = (typeof AI_OPTION_NAMES)[number]

/** The AI name of a stored gauge id (`score.gauge`, the trace's `gauge.selected`): both index GAUGE_TYPES. */
export const aiGaugeNameOf = (id: number) => AI_GAUGE_NAMES[id] ?? null

/** The AI name of a stored lane option (`score.random`). */
export const aiOptionNameOf = (random: string) => AI_OPTION_NAMES[RANDOM_OPTIONS.findIndex((option) => option === random)] ?? null

export const AI_GAUGE_OUTCOMES = ['cleared', 'failed', 'unknown'] as const
export const AI_RUN_ENDS = ['finished', 'gauge_empty', 'left'] as const
export const AI_COMPARE_TARGETS = ['best', 'rival'] as const
export const AI_COMPARE_MODES = ['segments', 'totals', 'none'] as const
export const AI_COMPARE_REASONS = [
    'no_other_play',
    'note_layout_differs',
    'rival_not_sharing',
    'rival_no_trace',
    'rival_no_record',
    'not_a_rival',
] as const

export type AiCompareMode = (typeof AI_COMPARE_MODES)[number]
export type AiCompareReason = (typeof AI_COMPARE_REASONS)[number]

const SCORE_ID_PATTERN = /^[A-Za-z0-9_-]{1,36}$/
const LOGIN_ID_PATTERN = /^[a-zA-Z0-9_-]{1,64}$/

export const aiScoreIdSchema = z.string().regex(SCORE_ID_PATTERN)
export const aiScoreParamSchema = z.object({ scoreId: aiScoreIdSchema })

export const aiCompareQuerySchema = z.object({
    against: z.enum(AI_COMPARE_TARGETS).default('best'),
    rival: z.string().regex(LOGIN_ID_PATTERN).optional(),
})

export type AiCompareQuery = z.infer<typeof aiCompareQuerySchema>

export const aiShareSettingPutSchema = z.object({ rival_segments: z.boolean() })
export const aiShareSettingSchema = aiShareSettingPutSchema.extend({ updated_at: z.number().int().nullable() })

export type AiShareSettingPutInput = z.infer<typeof aiShareSettingPutSchema>
export type AiShareSettingOutput = z.infer<typeof aiShareSettingSchema>

const evidenceSchema = z.enum(AI_EVIDENCE_GRADES)
const tagCountSchema = z.object({ tag: z.string(), notes: z.number().int() })

export const aiReportOutputSchema = z.object({
    headline: z.string(),
    strengths: z.array(aiInsightSchema),
    weaknesses: z.array(aiInsightSchema),
    habits: z.array(aiInsightSchema),
    practice_focus: z.array(z.object({ key: z.string(), action: z.string() })),
    caveats: z.array(z.string()),
})

const reportTagSchema = z.object({
    key: z.string(),
    tag: z.string(),
    notes: z.number().int(),
    plays: z.number().int(),
    auto_plays: z.number().int(),
    ex_ratio: z.number(),
    residual: z.number(),
    grade: evidenceSchema,
    leaning: z.enum(['weakness', 'strength']).nullable(),
})

const accuracyRowSchema = z.object({ key: z.string(), name: z.string(), notes: z.number().int(), ex_ratio: z.number().nullable() })

export const aiReportViewSchema = z.object({
    plays: z.object({
        analyzed: z.number().int(),
        measured: z.number().int(),
        stale: z.number().int(),
        failed: z.number().int(),
        from: z.number().int().nullable(),
        to: z.number().int().nullable(),
    }),
    tags: z.array(reportTagSchema),
    lanes: z.array(accuracyRowSchema.extend({ fast_share: z.number().nullable() })),
    density: z.array(accuracyRowSchema),
    fatigue: z.object({ key: z.string(), plays: z.number().int(), mean_delta: z.number().nullable() }),
    timing: z.object({ key: z.string(), notes: z.number().int(), mean_ms: z.number().nullable() }),
    options: z.array(z.object({ key: z.string(), option: z.string(), plays: z.number().int(), share: z.number() })),
    preferred: z.array(z.object({ key: z.string(), tag: z.string(), repeat_share: z.number(), all_share: z.number(), diff: z.number() })),
    lamps: z.object({ table: z.object({ id: z.string(), name: z.string(), symbol: z.string() }), levels: z.array(aiLevelProgressSchema) }).nullable(),
    score_rows: z.object({ plays: z.number().int(), fast: z.number().int(), slow: z.number().int() }),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiReportOutputSchema).nullable(),
})

export type AiReportOutput = z.infer<typeof aiReportOutputSchema>
export type AiReportView = z.infer<typeof aiReportViewSchema>

export const aiCoachOutputSchema = z.object({
    summary: z.string(),
    segments: z.array(z.object({ segment_id: z.string(), diagnosis: z.string(), advice: z.string() })),
    next_try: z.string(),
})

const coachSegmentSchema = z.object({
    key: z.string(),
    id: z.string(),
    from_ms: z.number().int(),
    to_ms: z.number().int(),
    chart_notes: z.number().int(),
    notes: z.number().int(),
    ex_ratio: z.number().nullable(),
    bp: z.number().int(),
    combo_breaks: z.number().int(),
    fast: z.number().int(),
    slow: z.number().int(),
    top_tags: z.array(tagCountSchema),
    gauge: z.number().nullable(),
})

const playChartSchema = z.object({ sha256: z.string(), md5: z.string().nullable(), title: z.string(), artist: z.string() })

const gaugeCurveViewSchema = z.object({
    gauge: z.enum(AI_GAUGE_NAMES),
    end: z.number(),
    cleared: z.boolean().nullable(),
    outcome: z.enum(AI_GAUGE_OUTCOMES),
})

export const aiCoachViewSchema = z.object({
    play: z.object({
        score_id: z.string(),
        played_at: z.number().int(),
        chart: playChartSchema,
        lamp: clearLampSchema,
        ex_score: z.number().int(),
        max_ex_score: z.number().int(),
        minbp: z.number().int(),
        random: z.string(),
        gauge: z.enum(GAUGE_TYPES),
        scratch_auto: z.boolean(),
    }),
    analysis: z
        .object({
            run_end: z.enum(AI_RUN_ENDS),
            overall: z.object({ notes: z.number().int(), ex_ratio: z.number().nullable() }),
            segments: z.array(coachSegmentSchema),
            problem_segments: z.array(z.string()),
            drops: z.array(z.object({ key: z.string(), from_ms: z.number().int(), to_ms: z.number().int(), drop: z.number() })),
            gauge: z.object({ selected: z.enum(AI_GAUGE_NAMES).nullable(), interval_ms: z.number().int(), curves: z.array(gaugeCurveViewSchema) }),
            pattern_join: z.string(),
        })
        .nullable(),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiCoachOutputSchema).nullable(),
})

export type AiCoachOutput = z.infer<typeof aiCoachOutputSchema>
export type AiCoachView = z.infer<typeof aiCoachViewSchema>

export const aiCompareOutputSchema = z.object({
    summary: z.string(),
    gaps: z.array(z.object({ segment_id: z.string(), explanation: z.string() })),
    takeaway: z.string(),
})

const judgeTotalsSchema = z.object({
    lamp: clearLampSchema,
    ex_score: z.number().int(),
    minbp: z.number().int(),
    pgreat: z.number().int(),
    great: z.number().int(),
    good: z.number().int(),
    bad: z.number().int(),
    poor: z.number().int(),
})

const compareSideSchema = z.object({
    who: z.enum(['me', 'rival']),
    login_id: z.string(),
    score_id: z.string().nullable(),
    played_at: z.number().int().nullable(),
    lamp: clearLampSchema,
    ex_score: z.number().int(),
    random: z.string().nullable(),
    scratch_auto: z.boolean(),
})

export const aiCompareViewSchema = z.object({
    against: z.enum(AI_COMPARE_TARGETS),
    base: compareSideSchema,
    target: compareSideSchema.nullable(),
    mode: z.enum(AI_COMPARE_MODES),
    reason: z.enum(AI_COMPARE_REASONS).nullable(),
    segments: z.array(
        z.object({
            key: z.string(),
            id: z.string(),
            from_ms: z.number().int(),
            to_ms: z.number().int(),
            chart_notes: z.number().int(),
            ex_diff: z.number().int().nullable(),
            bp_diff: z.number().int().nullable(),
            cumulative_ex_diff: z.number().int().nullable(),
            top_tags: z.array(tagCountSchema),
        }),
    ),
    gap_segments: z.array(z.string()),
    totals: z.object({ base: judgeTotalsSchema, target: judgeTotalsSchema }).nullable(),
    rivals: z.array(z.object({ login_id: z.string(), name: z.string(), allows_segments: z.boolean() })),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiCompareOutputSchema).nullable(),
})

export type AiCompareOutput = z.infer<typeof aiCompareOutputSchema>
export type AiCompareView = z.infer<typeof aiCompareViewSchema>

export const aiGaugeAdviceOutputSchema = z.object({
    summary: z.string(),
    gauge_advice: z.array(z.object({ gauge: z.enum(AI_GAUGE_NAMES), reasoning: z.string() })),
    option_advice: z.array(z.object({ option: z.enum(AI_OPTION_NAMES), reasoning: z.string() })),
})

export const aiGaugeAdviceViewSchema = z.object({
    plays: z.object({ analyzed: z.number().int(), stale: z.number().int(), failed: z.number().int(), unknown_runs: z.number().int() }),
    gauges: z.array(z.object({ key: z.string(), gauge: z.enum(AI_GAUGE_NAMES), plays: z.number().int(), cleared: z.number().int() })),
    rescues: z.array(
        z.object({
            key: z.string(),
            level: z.string(),
            played: z.enum(AI_GAUGE_NAMES),
            failed: z.number().int(),
            rescued_by: z.array(z.object({ gauge: z.enum(AI_GAUGE_NAMES), plays: z.number().int() })),
        }),
    ),
    options: z.array(
        z.object({
            key: z.string(),
            option: z.enum(AI_OPTION_NAMES),
            plays: z.number().int(),
            ex_ratio: z.number().nullable(),
            tags: z.array(reportTagSchema.pick({ tag: true, residual: true, grade: true, notes: true })),
        }),
    ),
    score_rows: z.array(z.object({ gauge: z.enum(AI_GAUGE_NAMES), plays: z.number().int(), cleared: z.number().int() })),
    notice: aiDataNoticeSchema,
    ai: aiInterpretationSchema(aiGaugeAdviceOutputSchema).nullable(),
})

export type AiGaugeAdviceOutput = z.infer<typeof aiGaugeAdviceOutputSchema>
export type AiGaugeAdviceView = z.infer<typeof aiGaugeAdviceViewSchema>
