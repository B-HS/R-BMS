import { z } from 'zod'
import {
    buildHashSchema,
    chartIdSchema,
    CLIENT_PLATFORM_LENGTH,
    clearLampSchema,
    columnTextSchema,
    countColumnSchema,
    extraSchema,
    floatColumnSchema,
    intColumnSchema,
    jsonTextSchema,
    judgeBreakdownSchema,
    playerIdSchema,
} from '@server/dto/common'

export const COURSE_HASH_MAX_LENGTH = 128
export const COURSE_NAME_MAX_LENGTH = 255
export const COURSE_ASSIST_TAG_MAX_LENGTH = 32
export const COURSE_ASSIST_TAGS_MAX = 16
export const COURSE_CHARTS_MAX = 64
export const COURSE_SUBMISSION_MAX_BYTES = 256 * 1024

export const courseTrophySchema = z.object({
    name: jsonTextSchema,
    scorerate: z.number().default(0),
    smissrate: z.number().default(0),
})

export const courseMetaSchema = z.object({
    course_hash: z.string().min(1).max(COURSE_HASH_MAX_LENGTH),
    name: z.string().max(COURSE_NAME_MAX_LENGTH).default(''),
    lntype: intColumnSchema.default(0),
    charts: z.array(chartIdSchema).default([]),
    constraint: z.array(jsonTextSchema).default([]),
    trophy: z.array(courseTrophySchema).default([]),
    extra: extraSchema,
})

export const courseMetaUpsertSchema = courseMetaSchema.extend({
    api_version: z.number().int().min(1).default(1),
})

export const courseSubmissionSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    course_hash: z.string().min(1).max(COURSE_HASH_MAX_LENGTH),
    player: playerIdSchema,
    lntype: z.number().int().default(0),
    clear: clearLampSchema,
    ex_score: countColumnSchema,
    max_ex_score: countColumnSchema.default(0),
    judge: judgeBreakdownSchema,
    max_combo: countColumnSchema.default(0),
    gauge_value: floatColumnSchema.default(0),
    minbp: countColumnSchema.default(0),
    charts: z.array(chartIdSchema).max(COURSE_CHARTS_MAX).default([]),
    trophy: z.string().max(32).nullish().default(null),
    played_at: z.number().int().min(0),
    scratch_auto: z.boolean().default(false),
    assist: z.array(jsonTextSchema.max(COURSE_ASSIST_TAG_MAX_LENGTH)).max(COURSE_ASSIST_TAGS_MAX).default([]),
    client_build_sha256: buildHashSchema,
    client_platform: columnTextSchema(CLIENT_PLATFORM_LENGTH).nullish().default(null),
    extra: extraSchema,
})

export type CourseMetaInput = z.infer<typeof courseMetaSchema>
export type CourseMetaUpsertInput = z.infer<typeof courseMetaUpsertSchema>
export type CourseSubmissionInput = z.infer<typeof courseSubmissionSchema>
