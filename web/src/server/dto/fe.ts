import { z } from 'zod'

const MAX_PAGE_LIMIT = 100
const DEFAULT_PAGE_LIMIT = 50
const MAX_FEED_LIMIT = 100
const DEFAULT_FEED_LIMIT = 20

export const CHART_SEARCH_SORTS = ['title', 'level', 'notes', 'recent'] as const
export const PLAYER_SORTS = ['rank_points', 'total_plays'] as const

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

export const playerLeaderboardQuerySchema = z.object({
    sort: z.enum(PLAYER_SORTS).default('rank_points'),
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
export type PlayerLeaderboardQuery = z.infer<typeof playerLeaderboardQuerySchema>
export type PlayerScoresQuery = z.infer<typeof playerScoresQuerySchema>
export type RivalPutInput = z.infer<typeof rivalPutSchema>
