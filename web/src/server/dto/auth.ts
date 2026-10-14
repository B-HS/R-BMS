import { z } from 'zod'
import { extraSchema, playerIdSchema } from '@server/dto/common'

export const MIN_PASSWORD_LENGTH = 8
export const MAX_PASSWORD_LENGTH = 128
const MAX_LOGIN_ID_LENGTH = 64
const LOGIN_ID_PATTERN = /^[a-zA-Z0-9_-]+$/
const RESERVED_LOGIN_IDS = ['guest']

const loginIdSchema = z
    .string()
    .min(1)
    .max(MAX_LOGIN_ID_LENGTH)
    .regex(LOGIN_ID_PATTERN)
    .refine((value) => !RESERVED_LOGIN_IDS.includes(value.toLowerCase()))

export const accountSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    id: loginIdSchema,
    password: z.string().min(MIN_PASSWORD_LENGTH).max(MAX_PASSWORD_LENGTH),
    email: z.email(),
    name: z.string().min(1).nullish().default(null),
})

export const loginSchema = z.object({
    api_version: z.number().int().min(1).default(1),
    id: z.string().min(1).max(MAX_LOGIN_ID_LENGTH),
    password: z.string().min(1).max(MAX_PASSWORD_LENGTH),
    email: z.email().nullish().default(null),
    name: z.string().nullish().default(null),
})

export const sessionLoginSchema = loginSchema.pick({ id: true, password: true })

export const sessionRegisterSchema = accountSchema.omit({ api_version: true })

export const authResponseSchema = z.object({
    token: z.string(),
    player: playerIdSchema,
    name: z.string(),
})

export const playerProfileSchema = z.object({
    id: z.string(),
    name: z.string(),
    total_plays: z.number().int().min(0),
    rank_points: z.number(),
    extra: extraSchema,
})

export const tokenIssueSchema = z.object({
    label: z.string().max(MAX_LOGIN_ID_LENGTH).default(''),
})

export type AccountInput = z.infer<typeof accountSchema>
export type LoginInput = z.infer<typeof loginSchema>
export type AuthResponseOutput = z.infer<typeof authResponseSchema>
export type PlayerProfileOutput = z.infer<typeof playerProfileSchema>
