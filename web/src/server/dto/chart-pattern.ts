import { z } from 'zod'
import { isMd5Hash, isSha256Hash } from '@server/dto/common'

export const CHART_PATTERN_FORMAT = 'rbms-chart-pattern'
export const CHART_PATTERN_FORMAT_VERSION = 1
export const CHART_PATTERN_MODE = 'BEAT_7K'

/** Lanes of a BEAT_7K document: 0 to 6 are the keys 1 to 7 and 7 is the scratch (`rbms-model` `Mode::BEAT_7K`). */
export const CHART_PATTERN_LANE_COUNT = 8
export const CHART_PATTERN_SCRATCH_LANE = 7

/** `notes.kind` as `rbms-cli chart-pattern` writes it (`apps/rbms-cli/src/chart_pattern/document.rs`). */
export const CHART_PATTERN_NOTE_KIND = { NORMAL: 0, LONG_HEAD: 1, LONG_END: 2, MINE: 3 } as const
export type ChartPatternNoteKind = (typeof CHART_PATTERN_NOTE_KIND)[keyof typeof CHART_PATTERN_NOTE_KIND]

const NOTE_KIND_MAX = CHART_PATTERN_NOTE_KIND.MINE

/** The largest pattern body the upload route reads. The largest BEAT_7K chart measured (2026-10-20) is about 1.6 MB of JSON. */
export const CHART_PATTERN_MAX_BYTES = 4 * 1024 * 1024

/** Uploads one admin token may make per minute. An upload of every BEAT_7K chart is about 8,000 requests. */
export const CHART_PATTERN_WRITE_RATE_LIMIT = { limit: 1200, windowMs: 60_000 } as const

/** The latest time a note or a tempo point may have: 24 hours in microseconds. No chart is longer, and it bounds every array built from times. */
export const CHART_PATTERN_MAX_TIME_US = 24 * 60 * 60 * 1_000_000

/** The most notes one pattern may hold. The largest BEAT_7K chart measured has about 64,000; the 4 MiB body limit alone would allow more than 500,000. */
export const CHART_PATTERN_MAX_NOTES = 200_000

/** Charts whose stored features are older than the current analysis version that one read (`readFeaturesMany`) computes again; the others are used as they are. */
export const CHART_PATTERN_READ_RECOMPUTE_MAX = 10

/** Charts one admin recompute request computes again: the default and the most it may ask for (each takes at most about 60 ms for the largest chart). */
export const CHART_PATTERN_RECOMPUTE_BATCH = { DEFAULT: 50, MAX: 200 } as const

/** Admin recompute requests one admin token may make per minute. */
export const CHART_PATTERN_RECOMPUTE_RATE_LIMIT = { limit: 30, windowMs: 60_000 } as const

export const CHART_PATTERN_DECLARED_LN = ['ln', 'cn', 'hcn'] as const

export const chartPatternMd5ParamSchema = z.object({ md5: z.string().refine(isMd5Hash, { message: 'md5 must be 32 lowercase hex characters' }) })

/** The body of the admin recompute: how many stale charts to compute again, and the md5 the previous batch ended at (null for the first batch). */
export const chartPatternRecomputeSchema = z.object({
    limit: z.number().int().min(1).max(CHART_PATTERN_RECOMPUTE_BATCH.MAX).default(CHART_PATTERN_RECOMPUTE_BATCH.DEFAULT),
    after: z.string().refine(isMd5Hash, { message: 'after must be 32 lowercase hex characters' }).nullable().default(null),
})

export type ChartPatternRecomputeInput = z.infer<typeof chartPatternRecomputeSchema>

/** The recompute body is a few bytes; this keeps the route from reading more. */
export const CHART_PATTERN_RECOMPUTE_MAX_BYTES = 1024

const tempoPointSchema = z.object({
    t_us: z.number().int().nonnegative().max(CHART_PATTERN_MAX_TIME_US),
    bpm: z.number(),
    stop_us: z.number().int(),
    scroll: z.number(),
})

const noteColumnSchema = z.array(z.number()).max(CHART_PATTERN_MAX_NOTES)

const noteColumnsSchema = z.object({
    time_us: noteColumnSchema,
    lane: noteColumnSchema,
    kind: noteColumnSchema,
    section: noteColumnSchema,
})

type NoteColumns = z.infer<typeof noteColumnsSchema>

type ColumnRule = { column: keyof NoteColumns; isValid: (value: number) => boolean; message: string }

const isLane = (value: number) => Number.isInteger(value) && value >= 0 && value < CHART_PATTERN_LANE_COUNT

const COLUMN_RULES: ColumnRule[] = [
    {
        column: 'time_us',
        isValid: (value) => Number.isSafeInteger(value) && value >= 0 && value <= CHART_PATTERN_MAX_TIME_US,
        message: `time_us must be an integer from 0 to ${CHART_PATTERN_MAX_TIME_US}`,
    },
    { column: 'lane', isValid: isLane, message: `lane must be an integer from 0 to ${CHART_PATTERN_LANE_COUNT - 1}` },
    {
        column: 'kind',
        isValid: (value) => Number.isInteger(value) && value >= 0 && value <= NOTE_KIND_MAX,
        message: `kind must be 0 to ${NOTE_KIND_MAX}`,
    },
    { column: 'section', isValid: (value) => value >= 0, message: 'section must not be negative' },
]

const isOrderedAfter = (notes: NoteColumns, index: number) => {
    const previousTime = notes.time_us[index - 1]
    const time = notes.time_us[index]
    return previousTime < time || (previousTime === time && notes.lane[index - 1] <= notes.lane[index])
}

const checkNoteColumns = (notes: NoteColumns, ctx: z.RefinementCtx) => {
    const length = notes.time_us.length
    const uneven = (Object.keys(notes) as (keyof NoteColumns)[]).find((column) => notes[column].length !== length)
    if (uneven) {
        ctx.addIssue({ code: 'custom', path: [uneven], message: `notes.${uneven} must have the length of notes.time_us (${length})` })
        return
    }
    for (const rule of COLUMN_RULES) {
        const badIndex = notes[rule.column].findIndex((value) => !rule.isValid(value))
        if (badIndex >= 0) {
            ctx.addIssue({ code: 'custom', path: [rule.column, badIndex], message: rule.message })
            return
        }
    }
    const unorderedIndex = notes.time_us.findIndex((_, index) => index > 0 && !isOrderedAfter(notes, index))
    if (unorderedIndex >= 0)
        ctx.addIssue({ code: 'custom', path: ['time_us', unorderedIndex], message: 'notes must be ordered by time_us and then lane' })
}

/**
 * One chart's pattern document as `rbms-cli chart-pattern` writes it (`apps/rbms-cli/src/chart_pattern/document.rs` is the source
 * of this shape). The notes are the chart's own layout with no play option applied, parallel columns ordered by time and lane.
 * `play_level` is null when the header has no integer level, and `total` is 0 when the chart states none.
 */
export const chartPatternDocumentSchema = z.object({
    format: z.literal(CHART_PATTERN_FORMAT),
    version: z.literal(CHART_PATTERN_FORMAT_VERSION),
    parser_version: z.number().int().positive(),
    md5: z.string().refine(isMd5Hash, { message: 'md5 must be 32 lowercase hex characters' }),
    sha256: z.string().refine(isSha256Hash, { message: 'sha256 must be 64 lowercase hex characters' }),
    mode: z.literal(CHART_PATTERN_MODE),
    declared_ln: z.enum(CHART_PATTERN_DECLARED_LN).nullable(),
    has_undefined_ln: z.boolean(),
    uses_control_flow: z.boolean(),
    init_bpm: z.number(),
    total: z.number().nonnegative(),
    rank: z.number().int(),
    play_level: z.number().int().nullable(),
    playable_notes: z.number().int().nonnegative(),
    notes: noteColumnsSchema.superRefine(checkNoteColumns),
    tempo: z.array(tempoPointSchema).refine((points) => points.every((point, index) => index === 0 || points[index - 1].t_us <= point.t_us), {
        message: 'tempo must be ordered by t_us',
    }),
})

export type ChartPatternDocument = z.infer<typeof chartPatternDocumentSchema>
export type ChartPatternTempoPoint = z.infer<typeof tempoPointSchema>

export const CHART_PATTERN_UPLOAD_OUTCOME = { CREATED: 'created', REPLACED: 'replaced', UNCHANGED: 'unchanged' } as const
export type ChartPatternUploadOutcome = (typeof CHART_PATTERN_UPLOAD_OUTCOME)[keyof typeof CHART_PATTERN_UPLOAD_OUTCOME]
