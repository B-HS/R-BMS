import { z } from 'zod'
import { chartIdSchema, extraSchema } from '@server/dto/common'
import { courseMetaSchema } from '@server/dto/course'

export const TABLE_ID_MAX_LENGTH = 64

export const tableFolderSchema = z.object({
    name: z.string().default(''),
    charts: z.array(chartIdSchema).default([]),
})

export const tableDataSchema = z.object({
    id: z.string().min(1).max(TABLE_ID_MAX_LENGTH),
    name: z.string().default(''),
    url: z.string().nullish().default(null),
    folders: z.array(tableFolderSchema).default([]),
    courses: z.array(courseMetaSchema).default([]),
    extra: extraSchema,
})

export const tableUpsertSchema = tableDataSchema.extend({
    api_version: z.number().int().min(1).default(1),
})

export type TableFolderInput = z.infer<typeof tableFolderSchema>
export type TableDataInput = z.infer<typeof tableDataSchema>
export type TableUpsertInput = z.infer<typeof tableUpsertSchema>
