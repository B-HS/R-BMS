import { z } from 'zod'
import { CHART_PATTERN_MAX_NOTES, CHART_PATTERN_MAX_TIME_US } from '@server/dto/chart-pattern'
import { intColumnSchema, isMd5Hash, isSha256Hash, randomOptionSchema } from '@server/dto/common'

/** The `format` tag of a play trace (`TRACE_FORMAT`, `crates/rbms-ir/src/dto/trace.rs`). */
export const TRACE_FORMAT = 'rbms-trace'

/** The generation of the play trace format this server reads (`TRACE_FORMAT_VERSION`). */
export const TRACE_FORMAT_VERSION = 1

/** The largest trace body the upload route reads: the client's `TRACE_MAX_BYTES`, which keeps a larger trace on the machine. */
export const TRACE_MAX_BYTES = 2 * 1024 * 1024

/** The most scroll setting changes and tempo points a trace carries (`TRACE_MAX_SPEED_CHANGES`, `TRACE_MAX_TEMPO_POINTS`). */
export const TRACE_MAX_SPEED_CHANGES = 4096
export const TRACE_MAX_TEMPO_POINTS = 4096

/** Trace uploads one account may make per minute. The client starts one every 2.5 seconds (`TRACE_SEND_INTERVAL`), 24 a minute. */
export const TRACE_UPLOAD_RATE_LIMIT = { limit: 30, windowMs: 60_000 } as const

/** The most notes and empty POORs one trace may hold: the bound a chart pattern has. The 2 MiB body holds about 95,000 notes. */
export const TRACE_MAX_NOTES = CHART_PATTERN_MAX_NOTES

/** Lanes are numbered from 0; no mode has this many. */
export const TRACE_LANE_LIMIT = 64

/** The most gauges a trace logs. The client logs nine (`GaugeIndex::ALL`). */
export const TRACE_MAX_GAUGES = 16

/** The most samples one gauge history holds: one every 500 ms over the longest allowed chart. */
export const TRACE_MAX_GAUGE_SAMPLES = 172_800

/** `notes.kind` (`TRACE_KIND_*`): a normal note, a long note's head, a plain LN end (never judged) and a CN or HCN end (judged). */
export const TRACE_NOTE_KIND = { NORMAL: 0, LONG_HEAD: 1, LONG_END: 2, CHARGE_END: 3 } as const

/** `notes.state`: 0 for an object nothing consumed, otherwise the judgement plus one, 1 PGREAT to 5 POOR. */
export const TRACE_NOTE_STATE = { NONE: 0, PGREAT: 1, GREAT: 2, GOOD: 3, BAD: 4, POOR: 5 } as const

/** The names a speed change's `field` takes (`TRACE_SPEED_FIELDS`). */
export const TRACE_SPEED_FIELDS = ['hispeed', 'constant', 'lane_cover', 'lift', 'hidden', 'offset_ms', 'auto_offset', 'green_ms', 'green'] as const

/**
 * Traces kept per account (ADR Q17): 5,000 runs and 512 MiB of trace JSON. Past either, a trace is still stored (no run's data is
 * thrown away); the account is flagged `over`, and `near` from NEAR_RATIO of either limit.
 */
export const TRACE_USER_QUOTA = { COUNT_MAX: 5000, TOTAL_BYTES_MAX: 512 * 1024 * 1024, NEAR_RATIO: 0.9 } as const

export const TRACE_QUOTA_LEVEL = { OK: 'ok', NEAR: 'near', OVER: 'over' } as const
export type TraceQuotaLevel = (typeof TRACE_QUOTA_LEVEL)[keyof typeof TRACE_QUOTA_LEVEL]

const SCORE_ID_MAX = 36
const MODE_LENGTH = 16
const NOTE_KIND_MAX = TRACE_NOTE_KIND.CHARGE_END
const NOTE_STATE_MAX = TRACE_NOTE_STATE.POOR

const safeIntegerSchema = z.number().int().min(-Number.MAX_SAFE_INTEGER).max(Number.MAX_SAFE_INTEGER)

const finiteSchema = z.number()

const columnSchema = z.array(z.number()).max(TRACE_MAX_NOTES)

const isLane = (value: number) => Number.isInteger(value) && value >= 0 && value < TRACE_LANE_LIMIT

const isTime = (value: number) => Number.isSafeInteger(value) && value >= 0 && value <= CHART_PATTERN_MAX_TIME_US

type ColumnRule<Column extends string> = { column: Column; isValid: (value: number) => boolean; message: string }

const checkColumns =
    <Column extends string>(rules: readonly ColumnRule<Column>[]) =>
    (columns: Record<Column, number[]>, ctx: z.RefinementCtx) => {
        const names = Object.keys(columns) as Column[]
        const length = columns[names[0]].length
        const uneven = names.find((name) => columns[name].length !== length)
        if (uneven) {
            ctx.addIssue({ code: 'custom', path: [uneven], message: `${uneven} must have the length of ${names[0]} (${length})` })
            return
        }
        for (const rule of rules) {
            const badIndex = columns[rule.column].findIndex((value) => !rule.isValid(value))
            if (badIndex >= 0) {
                ctx.addIssue({ code: 'custom', path: [rule.column, badIndex], message: rule.message })
                return
            }
        }
    }

const noteRules: readonly ColumnRule<'time_us' | 'lane' | 'kind' | 'state' | 'err_us'>[] = [
    { column: 'time_us', isValid: isTime, message: `time_us must be an integer from 0 to ${CHART_PATTERN_MAX_TIME_US}` },
    { column: 'lane', isValid: isLane, message: `lane must be an integer from 0 to ${TRACE_LANE_LIMIT - 1}` },
    {
        column: 'kind',
        isValid: (value) => Number.isInteger(value) && value >= 0 && value <= NOTE_KIND_MAX,
        message: `kind must be 0 to ${NOTE_KIND_MAX}`,
    },
    {
        column: 'state',
        isValid: (value) => Number.isInteger(value) && value >= 0 && value <= NOTE_STATE_MAX,
        message: `state must be 0 to ${NOTE_STATE_MAX}`,
    },
    { column: 'err_us', isValid: Number.isSafeInteger, message: 'err_us must be a safe integer' },
]

const emptyPoorRules: readonly ColumnRule<'time_us' | 'lane'>[] = [
    { column: 'time_us', isValid: Number.isSafeInteger, message: 'time_us must be a safe integer' },
    { column: 'lane', isValid: isLane, message: `lane must be an integer from 0 to ${TRACE_LANE_LIMIT - 1}` },
]

const notesSchema = z
    .object({ time_us: columnSchema, lane: columnSchema, kind: columnSchema, state: columnSchema, err_us: columnSchema })
    .superRefine(checkColumns(noteRules))

const emptyPoorsSchema = z.object({ time_us: columnSchema, lane: columnSchema }).superRefine(checkColumns(emptyPoorRules))

const gaugeSchema = z.object({
    interval_ms: z.number().int().positive(),
    selected: z
        .number()
        .int()
        .min(0)
        .max(TRACE_MAX_GAUGES - 1),
    series: z.array(z.array(finiteSchema).max(TRACE_MAX_GAUGE_SAMPLES)).max(TRACE_MAX_GAUGES),
})

const gaugeBoundsSchema = z.object({ min: finiteSchema, max: finiteSchema, border: finiteSchema })

const speedSchema = z.object({
    hispeed: finiteSchema,
    constant: z.boolean(),
    lane_cover: finiteSchema,
    lift: finiteSchema,
    hidden: finiteSchema,
    offset_ms: intColumnSchema,
    auto_offset: z.boolean(),
    green_ms: intColumnSchema,
    green: intColumnSchema,
})

const speedChangeSchema = z.object({ t_ms: safeIntegerSchema, field: z.enum(TRACE_SPEED_FIELDS), value: finiteSchema })

const tempoPointSchema = z.object({
    t_us: z.number().int().min(0).max(CHART_PATTERN_MAX_TIME_US),
    bpm: finiteSchema,
    stop_us: safeIntegerSchema,
    scroll: finiteSchema,
})

/**
 * The body of `POST /charts/{hash}/traces` (`PlayTrace`, `crates/rbms-ir/src/dto/trace.rs` is the source of this shape): what one
 * run was note by note as the client judged it, linked to its accepted score by `score_id`. The server checks it against the score
 * row and judges nothing again. Columns must be of one length; their order is the client's (time, then lane) and is not enforced.
 */
export const traceUploadSchema = z.object({
    format: z.literal(TRACE_FORMAT),
    version: z.literal(TRACE_FORMAT_VERSION),
    score_id: z.string().min(1).max(SCORE_ID_MAX),
    chart: z.object({
        md5: z.string().refine((value) => value === '' || isMd5Hash(value), { message: 'md5 must be 32 lowercase hex characters or empty' }),
        sha256: z.string().refine(isSha256Hash, { message: 'sha256 must be 64 lowercase hex characters' }),
    }),
    mode: z.string().max(MODE_LENGTH),
    lntype: intColumnSchema,
    random: randomOptionSchema,
    auto_lanes: z.array(z.boolean()).max(TRACE_LANE_LIMIT),
    notes: notesSchema,
    empty_poors: emptyPoorsSchema,
    gauge: gaugeSchema,
    gauge_bounds: z.array(gaugeBoundsSchema).max(TRACE_MAX_GAUGES),
    failed_at_ms: safeIntegerSchema.nullable(),
    play_time_ms: z.number().int().min(0).max(Number.MAX_SAFE_INTEGER),
    speed: z.object({ start: speedSchema, end: speedSchema, changes: z.array(speedChangeSchema).max(TRACE_MAX_SPEED_CHANGES) }),
    tempo: z.array(tempoPointSchema).max(TRACE_MAX_TEMPO_POINTS),
})

export type TraceUploadInput = z.infer<typeof traceUploadSchema>
export type TraceSpeedInput = z.infer<typeof speedSchema>

/** How many traces one admin recompute call looks at: `limit` from MIN to MAX, DEFAULT without one. */
export const TRACE_ANALYSIS_RECOMPUTE_BATCH = { MIN: 1, MAX: 200, DEFAULT: 50 } as const

/** `POST /api/admin/traces/recompute`: `after` is the `next_after` of the call before (none or null the first time). */
export const traceRecomputeSchema = z.object({
    limit: z
        .number()
        .int()
        .min(TRACE_ANALYSIS_RECOMPUTE_BATCH.MIN)
        .max(TRACE_ANALYSIS_RECOMPUTE_BATCH.MAX)
        .default(TRACE_ANALYSIS_RECOMPUTE_BATCH.DEFAULT),
    after: z.string().min(1).max(SCORE_ID_MAX).nullable().default(null),
})

/** Admin recompute requests one admin token may make per minute. */
export const TRACE_RECOMPUTE_RATE_LIMIT = { limit: 30, windowMs: 60_000 } as const

/** Share setting writes one account may make per minute. */
export const TRACE_SHARE_WRITE_RATE_LIMIT = { limit: 10, windowMs: 60_000 } as const
