import { z } from 'zod'
import { extraSchema, md5FieldSchema, sha256FieldSchema } from '@server/dto/common'

export const chartMetaSchema = z.object({
    md5: md5FieldSchema,
    sha256: sha256FieldSchema,
    title: z.string().default(''),
    subtitle: z.string().default(''),
    genre: z.string().default(''),
    artist: z.string().default(''),
    subartist: z.string().default(''),
    level: z.number().int().nullish().default(null),
    total: z.number().nullish().default(null),
    mode: z.string().default(''),
    lntype: z.number().int().default(0),
    judge: z.number().int().default(0),
    minbpm: z.number().int().default(0),
    maxbpm: z.number().int().default(0),
    notes: z.number().int().default(0),
    has_ln: z.boolean().default(false),
    has_cn: z.boolean().default(false),
    has_hcn: z.boolean().default(false),
    has_mine: z.boolean().default(false),
    has_random: z.boolean().default(false),
    has_stop: z.boolean().default(false),
    url: z.string().nullish().default(null),
    appendurl: z.string().nullish().default(null),
    extra: extraSchema,
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
