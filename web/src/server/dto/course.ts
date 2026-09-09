import { z } from 'zod'
import { chartIdSchema, clearLampSchema, extraSchema, judgeBreakdownSchema, playerIdSchema } from '@server/dto/common'

export const COURSE_HASH_MAX_LENGTH = 128

export const courseTrophySchema = z.object({
    name: z.string(),
    scorerate: z.number().default(0),
    smissrate: z.number().default(0),
})

export const courseMetaSchema = z.object({
    course_hash: z.string().min(1).max(COURSE_HASH_MAX_LENGTH),
    name: z.string().default(''),
    lntype: z.number().int().default(0),
    charts: z.array(chartIdSchema).default([]),
    constraint: z.array(z.string()).default([]),
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
    ex_score: z.number().int().min(0),
    max_ex_score: z.number().int().min(0).default(0),
    judge: judgeBreakdownSchema,
    max_combo: z.number().int().min(0).default(0),
    gauge_value: z.number().default(0),
    minbp: z.number().int().min(0).default(0),
    charts: z.array(chartIdSchema).default([]),
    trophy: z.string().max(32).nullish().default(null),
    played_at: z.number().int().min(0),
    extra: extraSchema,
})

export type CourseMetaInput = z.infer<typeof courseMetaSchema>
export type CourseMetaUpsertInput = z.infer<typeof courseMetaUpsertSchema>
export type CourseSubmissionInput = z.infer<typeof courseSubmissionSchema>
