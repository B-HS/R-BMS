import { z } from 'zod'
import { extraSchema } from '@server/dto/common'

export const SETTING_NAME_MAX_LENGTH = 64
export const SETTING_MAX_BYTES = 256 * 1024

export const settingPutSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    name: z.string().min(1).max(SETTING_NAME_MAX_LENGTH).nullish().default(null),
    format: z.string().max(16).default(''),
    content: z.string().max(SETTING_MAX_BYTES),
    updated_at: z.number().int().nullish().default(null),
    base_updated_at: z.number().int().nullish().default(null),
    extra: extraSchema,
})

export const settingBlobResponseSchema = z.object({
    name: z.string(),
    format: z.string(),
    content: z.string().max(SETTING_MAX_BYTES),
    updated_at: z.number().int(),
})

export type SettingPutInput = z.infer<typeof settingPutSchema>
export type SettingBlobOutput = z.infer<typeof settingBlobResponseSchema>
