import { z } from 'zod'
import { chartIdSchema, clearLampSchema, extraSchema, judgeBreakdownSchema, playOptionsSchema, playerIdSchema } from '@server/dto/common'

export const API_VERSION = 1

export const scoreSubmissionSchema = z.object({
    api_version: z.number().int().min(1).default(API_VERSION),
    chart: chartIdSchema,
    player: playerIdSchema,
    mode: z.string().default(''),
    clear: clearLampSchema,
    ex_score: z.number().int().min(0),
    max_ex_score: z.number().int().min(0).default(0),
    judge: judgeBreakdownSchema,
    max_combo: z.number().int().min(0).default(0),
    total_notes: z.number().int().min(0).default(0),
    passnotes: z.number().int().min(0).default(0),
    minbp: z.number().int().min(0).default(0),
    gauge_value: z.number().default(0),
    options: playOptionsSchema,
    played_at: z.number().int().min(0),
    client: z.string().default(''),
    replay_id: z.string().nullish().default(null),
    seed: z.number().int().default(0),
    judge_algorithm: z.string().default(''),
    rule: z.string().default(''),
    skin: z.string().default(''),
    client_build_sha256: z.string().nullish().default(null),
    client_platform: z.string().nullish().default(null),
    extra: extraSchema,
})

export const scoreRecordSchema = z.object({
    player: playerIdSchema,
    player_name: z.string(),
    clear: clearLampSchema,
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
})

export const bestQuerySchema = z.object({
    player: z.string().min(1),
    lnmode: z.coerce.number().int().min(0).max(2).optional(),
})

export type ScoreSubmissionInput = z.infer<typeof scoreSubmissionSchema>
export type ScoreRecordOutput = z.infer<typeof scoreRecordSchema>
export type SubmitResponseOutput = z.infer<typeof submitResponseSchema>
export type RankingQuery = z.infer<typeof rankingQuerySchema>
