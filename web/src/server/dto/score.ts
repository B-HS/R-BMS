import { z } from 'zod'
import {
    assistGradeSchema,
    buildHashSchema,
    chartIdSchema,
    CLIENT_PLATFORM_LENGTH,
    clearLampSchema,
    columnTextSchema,
    countColumnSchema,
    extraSchema,
    floatColumnSchema,
    includeScratchAutoSchema,
    judgeBreakdownSchema,
    playOptionsSchema,
    playerIdSchema,
    seedSchema,
} from '@server/dto/common'
import type { DisplayedDan } from '@server/dto/dan'

export const API_VERSION = 1

const CHART_TEXT_MAX = 255
const CHART_LEVEL_MAX = 1000
const MODE_LENGTH = 16
const JUDGE_ALGORITHM_LENGTH = 16
const RULE_LENGTH = 16
const SKIN_LENGTH = 64
const CLIENT_LENGTH = 64
const REPLAY_ID_MAX = 36

const lenientText = z
    .string()
    .nullish()
    .transform((value) => [...(value ?? '').trim()].slice(0, CHART_TEXT_MAX).join(''))

const lenientLevel = z
    .number()
    .nullish()
    .transform((value) => (typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= CHART_LEVEL_MAX ? value : null))

export const submissionChartSchema = chartIdSchema.extend({
    title: lenientText,
    artist: lenientText,
    level: lenientLevel,
})

export const scoreSubmissionSchema = z.object({
    api_version: z.number().int().min(1).default(API_VERSION),
    chart: submissionChartSchema,
    player: playerIdSchema,
    mode: columnTextSchema(MODE_LENGTH).default(''),
    clear: clearLampSchema,
    ex_score: countColumnSchema,
    max_ex_score: countColumnSchema.default(0),
    judge: judgeBreakdownSchema,
    max_combo: countColumnSchema.default(0),
    total_notes: countColumnSchema.default(0),
    passnotes: countColumnSchema.default(0),
    minbp: countColumnSchema.default(0),
    gauge_value: floatColumnSchema.default(0),
    options: playOptionsSchema,
    played_at: z.number().int().min(0),
    client: columnTextSchema(CLIENT_LENGTH).default(''),
    replay_id: z.string().max(REPLAY_ID_MAX).nullish().default(null),
    seed: seedSchema.default(0),
    judge_algorithm: columnTextSchema(JUDGE_ALGORITHM_LENGTH).default(''),
    rule: columnTextSchema(RULE_LENGTH).default(''),
    skin: columnTextSchema(SKIN_LENGTH).default(''),
    client_build_sha256: buildHashSchema,
    client_platform: columnTextSchema(CLIENT_PLATFORM_LENGTH).nullish().default(null),
    extra: extraSchema,
})

export const scoreRecordSchema = z.object({
    player: playerIdSchema,
    player_name: z.string(),
    clear: clearLampSchema,
    assist_grade: assistGradeSchema.default(0),
    ex_score: z.number().int().min(0),
    max_combo: z.number().int().min(0),
    minbp: z.number().int().min(0),
    rank: z.number().int().nullable(),
    played_at: z.number().int().min(0),
    lntype: z.number().int().default(0),
    option: z.number().int().default(0),
    total_notes: z.number().int().default(0),
    judge: judgeBreakdownSchema.nullish().default(null),
    extra: extraSchema,
})

export const submitResponseSchema = z.object({
    accepted: z.boolean(),
    rank: z.number().int().nullable(),
    previous_best: z.number().int().nullable(),
    message: z.string().nullable(),
    ranked: z.boolean(),
    flags: z.array(z.string()),
    is_new_best: z.boolean(),
    score_id: z.string().nullable(),
})

export const rankingQuerySchema = z.object({
    limit: z.coerce.number().int().min(1).max(500).default(50),
    page: z.coerce.number().int().min(1).default(1),
    rival_of: z.string().optional(),
    lnmode: z.coerce.number().int().min(0).max(2).optional(),
    include_scratch_auto: includeScratchAutoSchema,
})

export const bestQuerySchema = z.object({
    player: z.string().min(1),
    lnmode: z.coerce.number().int().min(0).max(2).optional(),
    include_scratch_auto: includeScratchAutoSchema,
})

export type ScoreSubmissionInput = z.infer<typeof scoreSubmissionSchema>
export type ScoreRecordOutput = z.infer<typeof scoreRecordSchema>
export type SubmitResponseOutput = z.infer<typeof submitResponseSchema>
export type RankingQuery = z.infer<typeof rankingQuerySchema>

/** A chart ranking row: a score record whose `extra` also carries the dans its player shows to others (empty when none). */
export type RankingRecordOutput = ScoreRecordOutput & { extra: { dans: DisplayedDan[] } }
