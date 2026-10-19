import { z } from 'zod'
import { includeScratchAutoSchema } from '@server/dto/common'

const MAX_PAGE_LIMIT = 100
const DEFAULT_PAGE_LIMIT = 50
const MAX_FEED_LIMIT = 100
const DEFAULT_FEED_LIMIT = 20

export const CHART_SEARCH_SORTS = ['title', 'level', 'notes', 'recent'] as const
export const PLAYER_SORTS = ['cleared', 'played', 'plays'] as const

export type PlayerSort = (typeof PLAYER_SORTS)[number]

const LEGACY_PLAYER_SORTS = { rank_points: 'cleared', total_plays: 'plays' } as const satisfies Record<string, PlayerSort>

const playerSortSchema = z.union([
    z.enum(PLAYER_SORTS),
    z.enum(['rank_points', 'total_plays']).transform((legacy): PlayerSort => LEGACY_PLAYER_SORTS[legacy]),
])

export const chartSearchQuerySchema = z.object({
    q: z.string().max(255).optional(),
    mode: z.string().max(16).optional(),
    level: z.coerce.number().int().min(0).optional(),
    sort: z.enum(CHART_SEARCH_SORTS).default('title'),
    page: z.coerce.number().int().min(1).default(1),
    limit: z.coerce.number().int().min(1).max(MAX_PAGE_LIMIT).default(DEFAULT_PAGE_LIMIT),
})

export const feedQuerySchema = z.object({
    limit: z.coerce.number().int().min(1).max(MAX_FEED_LIMIT).default(DEFAULT_FEED_LIMIT),
})

export const playerRecentQuerySchema = feedQuerySchema.extend({
    page: z.coerce.number().int().min(1).default(1),
})

export const chartLeaderboardQuerySchema = z.object({
    include_scratch_auto: includeScratchAutoSchema,
})

export const playerLeaderboardQuerySchema = z.object({
    sort: playerSortSchema.default('cleared'),
    include_scratch_auto: includeScratchAutoSchema,
    page: z.coerce.number().int().min(1).default(1),
    limit: z.coerce.number().int().min(1).max(MAX_PAGE_LIMIT).default(DEFAULT_PAGE_LIMIT),
})

export const playerScoresQuerySchema = z.object({
    since: z.coerce.number().int().min(0).optional(),
    mode: z.string().max(16).optional(),
    limit: z.coerce.number().int().min(1).max(MAX_PAGE_LIMIT).default(DEFAULT_PAGE_LIMIT),
    page: z.coerce.number().int().min(1).default(1),
})

export const rivalPutSchema = z.object({
    rivals: z.array(z.string().min(1).max(64)).max(MAX_PAGE_LIMIT),
})

export const tokenCreateSchema = z.object({
    label: z.string().max(64).default(''),
})

export type ChartSearchQuery = z.infer<typeof chartSearchQuerySchema>
export type FeedQuery = z.infer<typeof feedQuerySchema>
export type PlayerRecentQuery = z.infer<typeof playerRecentQuerySchema>
export type PlayerLeaderboardQuery = z.infer<typeof playerLeaderboardQuerySchema>
export type ChartLeaderboardQuery = z.infer<typeof chartLeaderboardQuerySchema>
export type PlayerScoresQuery = z.infer<typeof playerScoresQuerySchema>
export type RivalPutInput = z.infer<typeof rivalPutSchema>
