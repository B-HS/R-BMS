import { z } from 'zod'
import { chartIdSchema, extraSchema, gaugeTypeSchema, randomOptionSchema } from '@server/dto/common'

const MAX_REPLAY_LIST_LIMIT = 200
const DEFAULT_REPLAY_LIST_LIMIT = 50

export const replayEventSchema = z.object({
    t_us: z.number().int(),
    lane: z.number().int().min(0),
    press: z.boolean(),
})

export const replayUploadSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    format: z.string().default(''),
    chart: chartIdSchema.nullish().default(null),
    score_id: z.string().nullish().default(null),
    mode: z.string().default(''),
    random: randomOptionSchema.nullish().default(null),
    random_p2: randomOptionSchema.nullish().default(null),
    seed: z.number().int().nullish().default(null),
    lntype: z.number().int().default(0),
    offset_ms: z.number().int().default(0),
    judge_rate: z.number().int().default(0),
    scratch_auto: z.boolean().default(false),
    constant: z.boolean().default(false),
    gauge: gaugeTypeSchema.nullish().default(null),
    client_build_sha256: z.string().length(64).nullish().default(null),
    events: z.array(replayEventSchema),
    event_count: z.number().int().min(0).nullish().default(null),
    duration_us: z.number().int().min(0).nullish().default(null),
    size: z.number().int().min(0).nullish().default(null),
    extra: extraSchema,
})

export const replayListQuerySchema = z.object({
    player: z.string().min(1).optional(),
    limit: z.coerce.number().int().min(1).max(MAX_REPLAY_LIST_LIMIT).default(DEFAULT_REPLAY_LIST_LIMIT),
})

export type ReplayEventInput = z.infer<typeof replayEventSchema>
export type ReplayUploadInput = z.infer<typeof replayUploadSchema>
export type ReplayListQuery = z.infer<typeof replayListQuerySchema>
