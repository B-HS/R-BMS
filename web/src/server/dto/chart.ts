import { z } from 'zod'
import { extraSchema, intColumnSchema, md5FieldSchema, sha256FieldSchema } from '@server/dto/common'

const TEXT_MAX = 255
const MODE_MAX = 16
const URL_MAX = 512
const EXTRA_MAX_BYTES = 16 * 1024
export const CHART_UPSERT_MAX_BYTES = 64 * 1024

const boundedExtraSchema = extraSchema.refine((value) => JSON.stringify(value).length <= EXTRA_MAX_BYTES)

export const chartMetaSchema = z.object({
    md5: md5FieldSchema,
    sha256: sha256FieldSchema,
    title: z.string().max(TEXT_MAX).default(''),
    subtitle: z.string().max(TEXT_MAX).default(''),
    genre: z.string().max(TEXT_MAX).default(''),
    artist: z.string().max(TEXT_MAX).default(''),
    subartist: z.string().max(TEXT_MAX).default(''),
    level: intColumnSchema.nullish().default(null),
    total: z.number().nullish().default(null),
    mode: z.string().max(MODE_MAX).default(''),
    lntype: intColumnSchema.default(0),
    judge: intColumnSchema.default(0),
    minbpm: intColumnSchema.default(0),
    maxbpm: intColumnSchema.default(0),
    notes: intColumnSchema.default(0),
    has_ln: z.boolean().default(false),
    has_cn: z.boolean().default(false),
    has_hcn: z.boolean().default(false),
    has_mine: z.boolean().default(false),
    has_random: z.boolean().default(false),
    has_stop: z.boolean().default(false),
    url: z.string().max(URL_MAX).nullish().default(null),
    appendurl: z.string().max(URL_MAX).nullish().default(null),
    extra: boundedExtraSchema,
})

export const chartUpsertSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    chart: chartMetaSchema,
})

export type ChartMetaInput = z.infer<typeof chartMetaSchema>
export type ChartUpsertInput = z.infer<typeof chartUpsertSchema>

export const chartLookupQuerySchema = z.object({
    hash: z.string().optional(),
    md5: z.string().optional(),
    sha256: z.string().optional(),
})
