import { compose } from '@server/compose'
import { chartSearchQuerySchema, feedQuerySchema, playerLeaderboardQuerySchema, tokenCreateSchema } from '@server/dto/fe'
import { paginatedResponse, successResponse } from '@server/lib/api-response'
import { NextResponse } from 'next/server'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { withSession } from '@server/lib/with-session'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, withRateLimit } from '@server/lib/with-rate-limit'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'
import {
    getChartMetaCached,
    getChartRankingCached,
    getPlayerLeaderboardCached,
    getPlayerRecentCached,
    getPlayerStatsCached,
    getRecentActivityCached,
    getStatsSummaryCached,
    resolveChartSha256Cached,
    searchChartsCached,
} from '@server/service/read-cache'

type PlayerContext = { params: Promise<{ playerId: string }> }

const CREATED_STATUS = 201
const NO_CONTENT_STATUS = 204
const LEADERBOARD_LIMIT = 50

const envelope = <T>(data: T, status = 200) => NextResponse.json(successResponse(data), { status })

const withPublicRead = <TContext>(handler: (request: Request, context: TContext) => Promise<Response>) =>
    withErrorHandling(withRateLimit('fe-read', RATE_LIMIT.PUBLIC_READ)(handler))

const requireUserId = async (playerId: string) => {
    const userId = await compose().findUserIdByLoginId(playerId)
    if (!userId) throw createAppError(ERROR_CODE.IR_PLAYER_NOT_FOUND, { playerId })
    return userId
}

export const createFeChartSearchRoute = () =>
    withPublicRead(async (request: Request) => {
        const query = parseSearchParams(request, chartSearchQuerySchema)
        const { items, total } = await searchChartsCached(query)
        return NextResponse.json(paginatedResponse(items, { page: query.page, limit: query.limit, total }))
    })

export const createFeChartLeaderboardRoute = () =>
    withPublicRead(async (request: Request, context: { params: Promise<{ hash: string }> }) => {
        const { hash } = await context.params
        if (classifyChartHash(hash) === 'unknown') throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
        const resolved = await resolveChartSha256Cached(hash)
        if (!resolved) return envelope({ chart: null, ranking: [] })
        const [chart, ranking] = await Promise.all([
            getChartMetaCached(hash),
            getChartRankingCached({ chartSha256: resolved.sha256, limit: LEADERBOARD_LIMIT, page: 1 }),
        ])
        return envelope({ chart, ranking })
    })

export const createFeActivityRecentRoute = () =>
    withPublicRead(async (request: Request) => {
        const query = parseSearchParams(request, feedQuerySchema)
        return envelope(await getRecentActivityCached(query.limit))
    })

export const createFePlayerLeaderboardRoute = () =>
    withPublicRead(async (request: Request) => {
        const query = parseSearchParams(request, playerLeaderboardQuerySchema)
        const { items, total } = await getPlayerLeaderboardCached(query)
        return NextResponse.json(paginatedResponse(items, { page: query.page, limit: query.limit, total }))
    })

export const createFePlayerRecentRoute = () =>
    withPublicRead(async (request: Request, context: PlayerContext) => {
        const query = parseSearchParams(request, feedQuerySchema)
        const { playerId } = await context.params
        const userId = await requireUserId(playerId)
        return envelope(await getPlayerRecentCached({ loginId: playerId, userId, limit: query.limit }))
    })

export const createFePlayerStatsRoute = () =>
    withPublicRead(async (_request: Request, context: PlayerContext) => {
        const { playerId } = await context.params
        const userId = await requireUserId(playerId)
        return envelope(await getPlayerStatsCached({ loginId: playerId, userId }))
    })

export const createFeStatsSummaryRoute = () => withPublicRead(async () => envelope(await getStatsSummaryCached()))

export const createFeTokenListRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, feService } = compose()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(async (_authedRequest, _authedContext, user) =>
            envelope(await feService.listTokens(user.id)),
        )(request, context)
    })

export const createFeTokenCreateRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService } = compose()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest, _authedContext, user) => {
            const body = await parseJsonBody(authedRequest, tokenCreateSchema)
            const token = await authService.issueToken(user.id, body.label)
            return envelope({ token, created_at: Date.now() }, CREATED_STATUS)
        })(request, context)
    })

export const createFeTokenDeleteRoute = () =>
    withErrorHandling(async (request: Request, context: { params: Promise<{ tokenId: string }> }) => {
        const { authService, feService } = compose()
        return withSession({ resolveSessionUser: authService.resolveSessionUser })(
            async (_authedRequest, authedContext: { params: Promise<{ tokenId: string }> }, user) => {
                const { tokenId } = await authedContext.params
                const removed = await feService.deleteToken({ userId: user.id, tokenId })
                if (!removed) throw createAppError(ERROR_CODE.NOT_FOUND, { tokenId })
                return new NextResponse(null, { status: NO_CONTENT_STATUS })
            },
        )(request, context)
    })
