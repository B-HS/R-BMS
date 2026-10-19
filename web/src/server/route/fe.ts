import { compose } from '@server/compose'
import {
    chartLeaderboardQuerySchema,
    chartSearchQuerySchema,
    feedQuerySchema,
    playerLeaderboardQuerySchema,
    playerRecentQuerySchema,
    tokenCreateSchema,
} from '@server/dto/fe'
import { successResponse } from '@server/lib/api-response'
import { NextResponse } from 'next/server'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseJsonBody, parseSearchParams } from '@server/lib/parse-request'
import { withSession } from '@server/lib/with-session'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, withRateLimit } from '@server/lib/with-rate-limit'
import {
    readChartLeaderboard,
    readChartSearch,
    readPlayerLeaderboard,
    readPlayerRecent,
    readPlayerStats,
    readRecentActivity,
    readStatsSummary,
} from '@server/service/public-reads'

type PlayerContext = { params: Promise<{ playerId: string }> }

const CREATED_STATUS = 201
const NO_CONTENT_STATUS = 204

const envelope = <T>(data: T, status = 200) => NextResponse.json(successResponse(data), { status })

const pageEnvelope = <T>(page: { data: T[]; pagination: unknown }) => NextResponse.json({ success: true, ...page })

const withPublicRead = <TContext>(handler: (request: Request, context: TContext) => Promise<Response>) =>
    withErrorHandling(withRateLimit('fe-read', RATE_LIMIT.PUBLIC_READ)(handler))

export const createFeChartSearchRoute = () =>
    withPublicRead(async (request: Request) => {
        return pageEnvelope(await readChartSearch(parseSearchParams(request, chartSearchQuerySchema)))
    })

export const createFeChartLeaderboardRoute = () =>
    withPublicRead(async (request: Request, context: { params: Promise<{ hash: string }> }) => {
        const { hash } = await context.params
        return envelope(await readChartLeaderboard(hash, parseSearchParams(request, chartLeaderboardQuerySchema)))
    })

export const createFeActivityRecentRoute = () =>
    withPublicRead(async (request: Request) => {
        return envelope(await readRecentActivity(parseSearchParams(request, feedQuerySchema)))
    })

export const createFePlayerLeaderboardRoute = () =>
    withPublicRead(async (request: Request) => {
        return pageEnvelope(await readPlayerLeaderboard(parseSearchParams(request, playerLeaderboardQuerySchema)))
    })

export const createFePlayerRecentRoute = () =>
    withPublicRead(async (request: Request, context: PlayerContext) => {
        const query = parseSearchParams(request, playerRecentQuerySchema)
        const { playerId } = await context.params
        return pageEnvelope(await readPlayerRecent(playerId, query))
    })

export const createFePlayerStatsRoute = () =>
    withPublicRead(async (_request: Request, context: PlayerContext) => {
        const { playerId } = await context.params
        return envelope(await readPlayerStats(playerId))
    })

export const createFeStatsSummaryRoute = () => withPublicRead(async () => envelope(await readStatsSummary()))

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
