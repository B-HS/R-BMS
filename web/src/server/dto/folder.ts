import { z } from 'zod'
import { isMd5Hash, isSha256Hash } from '@server/dto/common'

const WRITE_MAX_BYTES = 512 * 1024

export const FOLDER_LIMIT = {
    FOLDERS_PER_USER: 50,
    ITEMS_PER_FOLDER: 2000,
    ITEMS_PER_USER: 20000,
    ITEMS_PER_REQUEST: 500,
    NAME_MAX: 64,
    DESCRIPTION_MAX: 255,
    TEXT_COPY_MAX: 200,
    NOTE_MAX: 100,
    IMPORT_LEVELS_MAX: 100,
    IMPORT_LEVEL_MAX: 32,
    TABLE_ID_MAX: 64,
    WRITE_MAX_BYTES,
} as const

export const FOLDER_ID_PREFIX = 'uf_'

const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f]/
const FOLDER_ID_PATTERN = /^uf_[0-9a-f]{30}$/

const singleLineText = (max: number) =>
    z
        .string()
        .transform((value) => value.normalize('NFC').trim())
        .refine((value) => value.length <= max && !CONTROL_CHARACTERS.test(value))

const hexHash = (isValid: (value: string) => boolean) =>
    z
        .string()
        .transform((value) => value.trim().toLowerCase())
        .refine(isValid)

const md5Schema = hexHash(isMd5Hash)

export const folderIdSchema = z.string().regex(FOLDER_ID_PATTERN)

export const folderNameSchema = singleLineText(FOLDER_LIMIT.NAME_MAX).refine((value) => value.length >= 1)

export const folderCreateSchema = z.object({
    name: folderNameSchema,
    description: singleLineText(FOLDER_LIMIT.DESCRIPTION_MAX).default(''),
})

export const folderUpdateSchema = z
    .object({
        name: folderNameSchema.optional(),
        description: singleLineText(FOLDER_LIMIT.DESCRIPTION_MAX).optional(),
    })
    .refine((value) => value.name !== undefined || value.description !== undefined)

export const folderOrderSchema = z.object({
    folder_ids: z.array(folderIdSchema).max(FOLDER_LIMIT.FOLDERS_PER_USER),
})

export const folderItemInputSchema = z.object({
    md5: md5Schema,
    sha256: hexHash((value) => value === '' || isSha256Hash(value)).default(''),
    title: z.string().default(''),
    artist: z.string().default(''),
    note: singleLineText(FOLDER_LIMIT.NOTE_MAX).default(''),
})

export const folderItemsAddSchema = z.object({
    items: z.array(folderItemInputSchema).min(1).max(FOLDER_LIMIT.ITEMS_PER_REQUEST),
})

export const folderItemsRemoveSchema = z.object({
    md5s: z.array(md5Schema).min(1).max(FOLDER_LIMIT.ITEMS_PER_FOLDER),
})

export const folderItemsOrderSchema = z.object({
    md5s: z.array(md5Schema).max(FOLDER_LIMIT.ITEMS_PER_FOLDER),
})

export const folderItemsReplaceSchema = z.object({
    items: z.array(folderItemInputSchema).max(FOLDER_LIMIT.ITEMS_PER_FOLDER),
    base_updated_at: z.number().int().min(0).optional(),
})

export const folderItemNoteSchema = z.object({
    note: singleLineText(FOLDER_LIMIT.NOTE_MAX),
})

export const folderImportSchema = z.object({
    table_id: z.string().min(1).max(FOLDER_LIMIT.TABLE_ID_MAX),
    levels: z.array(z.string().max(FOLDER_LIMIT.IMPORT_LEVEL_MAX)).max(FOLDER_LIMIT.IMPORT_LEVELS_MAX).optional(),
})

export const folderContainingParamSchema = z.object({ md5: md5Schema })

export type FolderCreateInput = z.infer<typeof folderCreateSchema>
export type FolderUpdateInput = z.infer<typeof folderUpdateSchema>
export type FolderItemInput = z.infer<typeof folderItemInputSchema>
export type FolderItemsAddInput = z.infer<typeof folderItemsAddSchema>
export type FolderItemsReplaceInput = z.infer<typeof folderItemsReplaceSchema>
export type FolderImportInput = z.infer<typeof folderImportSchema>

export const folderSummarySchema = z.object({
    id: z.string(),
    name: z.string(),
    description: z.string(),
    is_public: z.boolean(),
    position: z.number().int(),
    item_count: z.number().int(),
    updated_at: z.number().int(),
})

export const folderItemSchema = z.object({
    md5: z.string(),
    sha256: z.string(),
    title: z.string(),
    artist: z.string(),
    note: z.string(),
})

export const folderDetailSchema = folderSummarySchema.extend({ items: z.array(folderItemSchema) })

export const folderItemChangeSchema = z.object({
    added: z.number().int(),
    removed: z.number().int(),
    skipped: z.number().int(),
    item_count: z.number().int(),
    updated_at: z.number().int(),
})

export type FolderSummaryOutput = z.infer<typeof folderSummarySchema>
export type FolderItemOutput = z.infer<typeof folderItemSchema>
export type FolderDetailOutput = z.infer<typeof folderDetailSchema>
export type FolderItemChangeOutput = z.infer<typeof folderItemChangeSchema>
