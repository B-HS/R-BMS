import { z } from 'zod'
import { MAX_LEVEL_TEXT } from '@server/dto/table-snapshot'

const MD5_PATTERN = /^[0-9a-fA-F]{32}$/
const OPTIONAL_SHA256_PATTERN = /^([0-9a-fA-F]{64})?$/

const toLowerCase = (value: string) => value.toLowerCase()

/** The header of a difficulty table as its maker publishes it. Fields this server does not read pass through untouched. */
export const remoteHeaderSchema = z.object({
    name: z.string().min(1),
    symbol: z.string(),
    data_url: z.string().min(1),
    level_order: z.array(z.string()).optional(),
    last_update: z.string().optional(),
    course: z.unknown().optional(),
})

/** One chart of a published difficulty table: a level may be a string or a number, a hash may be written in either case. */
export const remoteEntrySchema = z.object({
    md5: z.string().regex(MD5_PATTERN).transform(toLowerCase),
    sha256: z.string().regex(OPTIONAL_SHA256_PATTERN).default('').transform(toLowerCase),
    level: z.union([z.string(), z.number()]).transform(String).pipe(z.string().min(1).max(MAX_LEVEL_TEXT)),
    title: z.string().default(''),
    artist: z.string().default(''),
    url: z.string().default(''),
    url_diff: z.string().default(''),
    name_diff: z.string().default(''),
})

export const remoteEntriesSchema = z.array(remoteEntrySchema)

export type RemoteHeader = z.infer<typeof remoteHeaderSchema>
export type RemoteEntry = z.infer<typeof remoteEntrySchema>
