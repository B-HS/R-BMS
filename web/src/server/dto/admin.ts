import { z } from 'zod'
import { sha256FieldSchema } from '@server/dto/common'

export const clientBuildSchema = z.object({
    sha256: sha256FieldSchema.refine((value) => value.length === 64, { message: 'sha256 must be 64 hex characters' }),
    version: z.string().max(32).default(''),
    platform: z.string().max(32).default(''),
    channel: z.string().max(16).default('stable'),
    released_at: z.number().int().min(0).default(0),
    trusted: z.boolean().default(true),
    note: z.string().max(255).default(''),
})

export type ClientBuildInput = z.infer<typeof clientBuildSchema>
