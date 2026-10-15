import { z } from 'zod'
import {
    chartIdSchema,
    columnTextSchema,
    countColumnSchema,
    extraSchema,
    gaugeTypeSchema,
    intColumnSchema,
    randomOptionSchema,
    seedSchema,
} from '@server/dto/common'

export const REPLAY_USER_QUOTA = { COUNT_MAX: 2000, TOTAL_BYTES_MAX: 512 * 1024 * 1024 } as const

const MAX_REPLAY_LIST_LIMIT = 200
const DEFAULT_REPLAY_LIST_LIMIT = 50

const FORMAT_LENGTH = 32
const MODE_LENGTH = 16
const SCORE_ID_MAX = 36

/** An instant or span in microseconds, kept to what JavaScript and the database's BIGINT hold exactly. */
const microsecondsSchema = z.number().int().min(-Number.MAX_SAFE_INTEGER).max(Number.MAX_SAFE_INTEGER)

export const replayEventSchema = z.object({
    t_us: microsecondsSchema,
    lane: z.number().int().min(0),
    press: z.boolean(),
})

export const replayUploadSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    format: columnTextSchema(FORMAT_LENGTH).default(''),
    chart: chartIdSchema.nullish().default(null),
    score_id: z.string().max(SCORE_ID_MAX).nullish().default(null),
    mode: columnTextSchema(MODE_LENGTH).default(''),
    random: randomOptionSchema.nullish().default(null),
    random_p2: randomOptionSchema.nullish().default(null),
    seed: seedSchema.nullish().default(null),
    lntype: intColumnSchema.default(0),
    offset_ms: intColumnSchema.default(0),
    judge_rate: intColumnSchema.default(0),
    scratch_auto: z.boolean().default(false),
    constant: z.boolean().default(false),
    gauge: gaugeTypeSchema.nullish().default(null),
    client_build_sha256: z.string().length(64).nullish().default(null),
    events: z.array(replayEventSchema),
    event_count: countColumnSchema.nullish().default(null),
    duration_us: microsecondsSchema.min(0).nullish().default(null),
    size: countColumnSchema.nullish().default(null),
    extra: extraSchema,
})

export const replayListQuerySchema = z.object({
    player: z.string().min(1).optional(),
    limit: z.coerce.number().int().min(1).max(MAX_REPLAY_LIST_LIMIT).default(DEFAULT_REPLAY_LIST_LIMIT),
})

export type ReplayEventInput = z.infer<typeof replayEventSchema>
export type ReplayUploadInput = z.infer<typeof replayUploadSchema>
export type ReplayListQuery = z.infer<typeof replayListQuerySchema>
