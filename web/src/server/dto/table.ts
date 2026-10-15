import { z } from 'zod'
import { chartIdSchema, extraSchema } from '@server/dto/common'
import { courseMetaSchema } from '@server/dto/course'

export const TABLE_ID_MAX_LENGTH = 64
export const TABLE_NAME_MAX_LENGTH = 255
export const TABLE_URL_MAX_LENGTH = 512
export const TABLE_FOLDER_ID_MAX_LENGTH = 64
export const TABLE_FOLDER_NAME_MAX_LENGTH = 255

/** The id a table's folder is stored under: the table id and the folder's position in the table. */
export const folderIdFor = (tableId: string, position: number) => `${tableId}#${position}`

export const tableFolderSchema = z.object({
    name: z.string().max(TABLE_FOLDER_NAME_MAX_LENGTH).default(''),
    charts: z.array(chartIdSchema).default([]),
})

export const tableDataSchema = z
    .object({
        id: z.string().min(1).max(TABLE_ID_MAX_LENGTH),
        name: z.string().max(TABLE_NAME_MAX_LENGTH).default(''),
        url: z.string().max(TABLE_URL_MAX_LENGTH).nullish().default(null),
        folders: z.array(tableFolderSchema).default([]),
        courses: z.array(courseMetaSchema).default([]),
        extra: extraSchema,
    })
    .refine((table) => table.folders.length === 0 || folderIdFor(table.id, table.folders.length - 1).length <= TABLE_FOLDER_ID_MAX_LENGTH, {
        path: ['folders'],
        message: 'the table id and folder count make a folder id longer than its column',
    })

export const tableUpsertSchema = tableDataSchema.safeExtend({
    api_version: z.number().int().min(1).default(1),
})

export type TableFolderInput = z.infer<typeof tableFolderSchema>
export type TableDataInput = z.infer<typeof tableDataSchema>
export type TableUpsertInput = z.infer<typeof tableUpsertSchema>
