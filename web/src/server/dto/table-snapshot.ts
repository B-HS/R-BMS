import { z } from 'zod'
import { clearLampSchema, isMd5Hash } from '@server/dto/common'

const MAX_SEARCH_LIMIT = 100
const DEFAULT_SEARCH_LIMIT = 50
const MAX_SEARCH_TEXT = 100
const MAX_LEVEL_TEXT = 32

export const snapshotHeaderSchema = z.object({
    name: z.string().min(1),
    symbol: z.string().default(''),
    data_url: z.string().min(1),
    level_order: z.array(z.string()),
    last_update: z.string().default(''),
    original_url: z.string().default(''),
})

export const snapshotEntrySchema = z.object({
    md5: z.string().refine(isMd5Hash),
    level: z.string(),
    title: z.string().default(''),
    artist: z.string().default(''),
    url: z.string().default(''),
    url_diff: z.string().default(''),
    name_diff: z.string().default(''),
    comment: z.string().default(''),
    lr2_bmsid: z.string().default(''),
})

export const snapshotEntriesSchema = z.array(snapshotEntrySchema)

export const snapshotSourceSchema = z.object({
    origin: z.object({ page_url: z.string(), header_url: z.string() }),
    fetched_at: z.string(),
    snapshot: z.object({ header_file_sha256: z.string(), data_file_sha256: z.string(), entries: z.number().int(), levels: z.number().int() }),
})

export const tableEntriesQuerySchema = z.object({
    q: z.string().max(MAX_SEARCH_TEXT).optional(),
    level: z.string().max(MAX_LEVEL_TEXT).optional(),
    page: z.coerce.number().int().min(1).default(1),
    limit: z.coerce.number().int().min(1).max(MAX_SEARCH_LIMIT).default(DEFAULT_SEARCH_LIMIT),
})

export const tableIdParamSchema = z.string().regex(/^[a-z0-9][a-z0-9-]{0,63}$/)

export type SnapshotHeader = z.infer<typeof snapshotHeaderSchema>
export type SnapshotEntry = z.infer<typeof snapshotEntrySchema>
export type SnapshotSource = z.infer<typeof snapshotSourceSchema>
export type TableEntriesQuery = z.infer<typeof tableEntriesQuerySchema>

export const tableEntryOutputSchema = z.object({
    md5: z.string(),
    level: z.string(),
    title: z.string(),
    artist: z.string(),
    url: z.string(),
    url_diff: z.string(),
    name_diff: z.string(),
    comment: z.string(),
})

export const tableSummaryOutputSchema = z.object({
    id: z.string(),
    name: z.string(),
    symbol: z.string(),
    entry_count: z.number().int(),
    level_count: z.number().int(),
    last_update: z.string(),
    original_url: z.string(),
    source: z.object({ page_url: z.string(), fetched_at: z.string() }),
    levels: z.array(z.object({ level: z.string(), count: z.number().int() })),
    standard_paths: z.object({ page: z.string(), header: z.string(), data: z.string() }),
})

export const tableDetailOutputSchema = z.object({
    table: tableSummaryOutputSchema,
    levels: z.array(z.object({ level: z.string(), entries: z.array(tableEntryOutputSchema) })),
})

export const tableRecordOutputSchema = z.object({
    clear: clearLampSchema,
    ex_score: z.number().int(),
    minbp: z.number().int(),
    max_combo: z.number().int(),
    total_notes: z.number().int(),
    played_at: z.number().int(),
})

export const tableRecordsOutputSchema = z.object({
    table_id: z.string(),
    records: z.record(z.string(), tableRecordOutputSchema),
})

export type TableEntryOutput = z.infer<typeof tableEntryOutputSchema>
export type TableSummaryOutput = z.infer<typeof tableSummaryOutputSchema>
export type TableDetailOutput = z.infer<typeof tableDetailOutputSchema>
export type TableRecordOutput = z.infer<typeof tableRecordOutputSchema>
export type TableRecordsOutput = z.infer<typeof tableRecordsOutputSchema>
