import { z } from 'zod'
import { weaknessKeySchema } from '@server/service/domain/analysis/weakness-key'
import type { Locale } from '@shared/i18n/locale'

/** The note patterns the training chart synthesiser can lay out (plan 2026-11-04 3.3, 3.6). */
export const TRAINING_PATTERN_KINDS = [
    'stream',
    'chord_stream',
    'trill',
    'stair',
    'jack',
    'chord_jack',
    'scratch_solo',
    'scratch_mixed',
    'scratch_run',
    'ln_hold',
    'ln_release',
    'burst',
    'rest',
] as const
export type TrainingPatternKind = (typeof TRAINING_PATTERN_KINDS)[number]

/** Which keys a section leans on (plan 3.6 lane candidates). */
export const TRAINING_LANE_HINTS = ['all', 'left', 'right', 'center', 'wide', 'adjacent'] as const
export type TrainingLaneHint = (typeof TRAINING_LANE_HINTS)[number]

/** How hard a training chart is against the player's density profile (plan 3.5 NPS range). */
export const TRAINING_INTENSITIES = ['build', 'push', 'over'] as const
export type TrainingIntensity = (typeof TRAINING_INTENSITIES)[number]

export const TRAINING_LIMIT = {
    CHARTS_PER_USER: 20,
    DESCRIPTION_MAX: 255,
    INTENT_MAX: 100,
    ADVICE_MAX: 300,
    SECTIONS_MIN: 3,
    SECTIONS_MAX: 12,
    NOTE_MAX: 100,
    WRITE_MAX_BYTES: 1024,
} as const

/** The intensity as the public `#TITLE` spells it. The title names no weakness, since plays of a training chart are public (plan 3.7). */
export const TRAINING_INTENSITY_TITLE = { build: 'BUILD', push: 'PUSH', over: 'OVER' } as const satisfies Record<TrainingIntensity, string>

/** The name the user's training folder is made with, in the language of the screen it is made from, and never renamed after (plan 5.3). */
export const TRAINING_FOLDER_NAME = { ko: '훈련곡', en: 'Training', ja: 'トレーニング' } as const satisfies Record<Locale, string>

export const TRAINING_FOLDER_DESCRIPTION = {
    ko: 'AI 분석으로 만든 훈련곡입니다. R-BMS 선곡 화면의 MY FOLDERS 에서 바로 받아 플레이할 수 있습니다.',
    en: 'Training charts made from your AI analysis. Download and play them from MY FOLDERS on the R-BMS song select.',
    ja: 'AI分析から作ったトレーニング譜面です。R-BMSの選曲画面のMY FOLDERSからそのままダウンロードしてプレイできます。',
} as const satisfies Record<Locale, string>

/**
 * The plan a provider writes for a training chart (plan 3.3). Every property is required and none is nullable, so the provider's
 * structured output accepts it; ranges are not part of the provider schema and are enforced by `correctTrainingPlan` afterwards.
 */
export const aiTrainingPlanOutputSchema = z.object({
    description: z.string().trim().max(TRAINING_LIMIT.DESCRIPTION_MAX),
    base_bpm: z.number(),
    sections: z
        .array(
            z.object({
                focus_key: z.string(),
                kind: z.enum(TRAINING_PATTERN_KINDS),
                weight: z.number(),
                target_nps: z.number(),
                lane_hint: z.enum(TRAINING_LANE_HINTS),
                chord_size: z.number(),
                scratch_ratio: z.number(),
                ln_ratio: z.number(),
                bpm_ratio: z.number(),
                intent: z.string().trim().max(TRAINING_LIMIT.INTENT_MAX),
            }),
        )
        .min(1)
        .max(TRAINING_LIMIT.SECTIONS_MAX),
    advice: z.string().trim().max(TRAINING_LIMIT.ADVICE_MAX),
})

export type AiTrainingPlanOutput = z.infer<typeof aiTrainingPlanOutputSchema>
export type AiTrainingPlanSection = AiTrainingPlanOutput['sections'][number]

/** The literal a request names when the server is to pick the weaknesses a chart trains (plan 3.1). */
export const TRAINING_AUTO_FOCUS = 'auto'

const TRAINING_CHART_ID_PATTERN = /^tc_[0-9a-f]{32}$/

/** A training chart id as the paths carry it: `tc_` and 32 lower-case hex characters (plan 4.1). */
export const trainingChartIdSchema = z.string().regex(TRAINING_CHART_ID_PATTERN)

/** `POST /api/fe/me/ai/training-charts` (plan 3.1): the weakness to train, or `auto`, and how hard. Nothing else is taken. */
export const trainingChartCreateSchema = z
    .object({
        focus_key: z.union([z.literal(TRAINING_AUTO_FOCUS), weaknessKeySchema]),
        intensity: z.enum(TRAINING_INTENSITIES),
    })
    .strict()

export type TrainingChartCreateInput = z.infer<typeof trainingChartCreateSchema>

/** The public path a training chart's zip is served at (plan 5.1); the folder and training domains build their links with it. */
export const trainingDownloadPathOf = (id: string) => `/api/training/${id}/chart.zip`

/** `training_chart.request` (plan 4.1): what was asked and the provider's plan as refined, kept private to the owner. */
export const trainingChartRequestSchema = z.object({
    focus_key: z.string(),
    intensity: z.enum(TRAINING_INTENSITIES),
    plan: aiTrainingPlanOutputSchema,
    advice: z.string(),
    description: z.string(),
})

export type TrainingChartRequest = z.infer<typeof trainingChartRequestSchema>

const evidenceValueSchema = z.string()

/** What the profile said of one focus key when the chart was made: the "before" of the comparison (plan 7.2 `BeforeAfter`). */
export const trainingTargetSchema = z.object({
    value: z.number(),
    unit: z.string(),
    notes: z.number().int(),
    charts: z.number().int(),
    evidence: evidenceValueSchema,
})

/** `training_chart.profile_snapshot` (plan 4.1): the profile the chart was planned from and its value of every focus key. */
export const trainingProfileSnapshotSchema = z.object({
    profile_version: z.number().int(),
    fingerprint: z.string(),
    computed_at: z.number().int(),
    targets: z.record(z.string(), trainingTargetSchema),
})

export type TrainingProfileSnapshot = z.infer<typeof trainingProfileSnapshotSchema>

/** One training chart in the owner's list (plan 7.2 `TrainingChartSummary`); `download_url` is built on the request's origin. */
export const trainingChartSummarySchema = z.object({
    id: z.string(),
    title: z.string(),
    created_at: z.number().int(),
    focus_keys: z.array(z.string()),
    kinds: z.array(z.string()),
    note_count: z.number().int(),
    bpm: z.number().int(),
    play_level: z.number().int(),
    length_ms: z.number().int(),
    download_url: z.string(),
    plays: z.number().int(),
    best_ex_ratio: z.number().nullable(),
    last_played_at: z.number().int().nullable(),
})

/** `GET /api/fe/me/ai/training-charts`: the owner's charts, newest first, the room left and whether an AI key is registered. */
export const trainingChartListSchema = z.object({
    charts: z.array(trainingChartSummarySchema),
    used: z.number().int(),
    limit: z.number().int(),
    can_create: z.boolean(),
    needs_key: z.boolean(),
})

/** A section's result in one play, from the play's analysis (plan 2.2.8). */
export const trainingPlaySectionSchema = z.object({
    index: z.number().int(),
    ex_ratio: z.number().nullable(),
    bp: z.number().int(),
    mean_err_ms: z.number().nullable(),
    fast: z.number().int(),
    slow: z.number().int(),
})

/** One play of the chart by its owner; `sections` is null when the play has no readable trace. */
export const trainingPlaySchema = z.object({
    score_id: z.string(),
    played_at: z.number().int(),
    clear: z.number().int(),
    ex_score: z.number().int(),
    ex_ratio: z.number().nullable(),
    bp: z.number().int(),
    random: z.string(),
    sections: z.array(trainingPlaySectionSchema).nullable(),
})

/** One focus key before the chart was made and over the plays since (plan 7.2 `BeforeAfter`); `after` is null below the sample conditions. */
export const trainingBeforeAfterSchema = z.object({
    focus_key: z.string(),
    unit: z.string(),
    before: trainingTargetSchema,
    after: trainingTargetSchema.extend({ plays: z.number().int() }).nullable(),
    delta: z.number().nullable(),
})

/** A value the server changed from the provider's plan (the stored spec's `corrections`). */
export const trainingCorrectionViewSchema = z.object({
    section: z.number().int().nullable(),
    field: z.string(),
    from: z.union([z.string(), z.number()]).nullable(),
    to: z.union([z.string(), z.number()]).nullable(),
})

/**
 * `GET /api/fe/me/ai/training-charts/[id]` (plan 7.2 `TrainingChartDetail`). `plays` holds the newest TRAINING_PLAYS_SHOWN plays,
 * and `play_count` how many there are in all (the summary's count).
 */
export const trainingChartDetailSchema = trainingChartSummarySchema.omit({ plays: true }).extend({
    play_count: z.number().int(),
    description: z.string(),
    advice: z.string(),
    sections: z.array(
        z.object({
            index: z.number().int(),
            focus_key: z.string(),
            kind: z.enum(TRAINING_PATTERN_KINDS),
            from_ms: z.number().int(),
            to_ms: z.number().int(),
            target_nps: z.number(),
            actual_nps: z.number(),
            lane_hint: z.enum(TRAINING_LANE_HINTS),
            intent: z.string(),
        }),
    ),
    corrections: z.array(trainingCorrectionViewSchema),
    plays: z.array(trainingPlaySchema),
    before_after: z.array(trainingBeforeAfterSchema),
})

/** What deleting a chart takes away and what it leaves (plan 7.2 `removal`), counted before anything is deleted. */
export const trainingRemovalSchema = z.object({
    removes: z.object({ training_chart: z.number().int(), folder_items: z.number().int() }),
    keeps: z.object({
        chart_pattern: z.number().int(),
        scores: z.number().int(),
        traces: z.number().int(),
        replays: z.number().int(),
    }),
})

export const trainingRemovedSchema = trainingRemovalSchema.extend({ removed: z.literal(true) })

export type TrainingChartSummary = z.infer<typeof trainingChartSummarySchema>
export type TrainingChartList = z.infer<typeof trainingChartListSchema>
export type TrainingPlay = z.infer<typeof trainingPlaySchema>
export type TrainingBeforeAfter = z.infer<typeof trainingBeforeAfterSchema>
export type TrainingChartDetail = z.infer<typeof trainingChartDetailSchema>
export type TrainingRemoval = z.infer<typeof trainingRemovalSchema>
