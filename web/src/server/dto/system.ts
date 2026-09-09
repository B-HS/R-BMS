import { z } from 'zod'

export const serverCapabilitiesSchema = z.object({
    ranking: z.boolean().default(false),
    player_best: z.boolean().default(false),
    rivals: z.boolean().default(false),
    courses: z.boolean().default(false),
    replays: z.boolean().default(false),
    tables: z.boolean().default(false),
    settings_sync: z.boolean().default(false),
    accounts: z.boolean().default(false),
    lr2ir_compat: z.boolean().default(false),
})

export const serverInfoSchema = z.object({
    name: z.string(),
    version: z.string(),
    ir_compat: z.string(),
    capabilities: serverCapabilitiesSchema,
})

export const versionInfoSchema = z.object({
    api_version: z.number().int(),
    server: z.string(),
    commit: z.string().nullable(),
})

export type ServerInfoOutput = z.infer<typeof serverInfoSchema>
export type ServerCapabilitiesOutput = z.infer<typeof serverCapabilitiesSchema>
