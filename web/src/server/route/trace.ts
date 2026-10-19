import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { aiShareSettingPutSchema } from '@server/dto/ai-play'
import {
    TRACE_MAX_BYTES,
    TRACE_RECOMPUTE_RATE_LIMIT,
    TRACE_SHARE_WRITE_RATE_LIMIT,
    TRACE_UPLOAD_RATE_LIMIT,
    traceRecomputeSchema,
    traceUploadSchema,
} from '@server/dto/trace'
import { irRaw, successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { guardBodySize, parseLimitedJsonBody } from '@server/lib/parse-request'
import { AI_LIMIT } from '@server/dto/ai'
import { parseBearerToken } from '@server/lib/token'
import { withAdminBearer } from '@server/lib/with-admin-bearer'
import { withApiToken } from '@server/lib/with-api-token'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withSession } from '@server/lib/with-session'
import { withSummarizedErrors } from '@server/lib/with-summarized-errors'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'
import { TRACE_UPLOAD_OUTCOME, type TraceService } from '@server/service/domain/trace/trace.service'

export type TraceRouteDeps = {
    authService: Pick<AuthService, 'resolveApiTokenUser' | 'resolveSessionUser'>
    traceService: Pick<TraceService, 'upload' | 'usageOf' | 'adminUsage' | 'shareSettingOf' | 'putShareSetting' | 'recomputeStale'>
}

type HashContext = { params: Promise<{ hash: string }> }

type TraceHandler<TContext> = (request: Request, context: TContext, user: AuthUser, deps: TraceRouteDeps) => Promise<Response>

const CREATED_STATUS = 201
const OK_STATUS = 200
const PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

const TRACE_ROUTE_FAILED = 'trace route failed'

const privateJson = (body: unknown) => NextResponse.json(body, { headers: PRIVATE_HEADERS })

/**
 * The play trace routes. The client uploads with its API token (`POST /charts/{hash}/traces`); a trace is never read back by any
 * route. The account's own quota view and share setting (session) and the admin usage and recompute (admin API token) answer counts and
 * settings only. An unexpected error is
 * logged by its class and driver code only (`withSummarizedErrors`): a database error message carries the trace body it bound.
 */
export const createTraceRoutes = (resolveDeps: () => TraceRouteDeps = compose, waitForRequest: () => Promise<void> = connection) => {
    const withDeps = <TContext>(run: (request: Request, context: TContext, deps: TraceRouteDeps) => Promise<Response>) =>
        withSummarizedErrors(TRACE_ROUTE_FAILED)(async (request: Request, context: TContext) => {
            await waitForRequest()
            return run(request, context, resolveDeps())
        })

    const withToken = <TContext>(handler: TraceHandler<TContext>) =>
        withErrorHandling(
            withDeps<TContext>(async (request, context, deps) => {
                guardBodySize(request, TRACE_MAX_BYTES)
                if (!parseBearerToken(request)) throw createAppError(ERROR_CODE.UNAUTHORIZED)
                return withApiToken({ resolveApiTokenUser: deps.authService.resolveApiTokenUser })((authedRequest, authedContext: TContext, user) =>
                    handler(authedRequest, authedContext, user, deps),
                )(request, context)
            }),
        )

    const withUser = <TContext>(handler: TraceHandler<TContext>) =>
        withErrorHandling(
            withDeps<TContext>(async (request, context, deps) =>
                withSession({ resolveSessionUser: deps.authService.resolveSessionUser })((authedRequest, authedContext: TContext, user) =>
                    handler(authedRequest, authedContext, user, deps),
                )(request, context),
            ),
        )

    const withAdminToken = <TContext>(handler: TraceHandler<TContext>) =>
        withErrorHandling(
            withDeps<TContext>(async (request, context, deps) =>
                withAdminBearer({ resolveApiTokenUser: deps.authService.resolveApiTokenUser })((authedRequest, authedContext: TContext, user) =>
                    handler(authedRequest, authedContext, user, deps),
                )(request, context),
            ),
        )

    return {
        upload: withToken<HashContext>(async (request, context, user, deps) => {
            enforceUserRateLimit('trace-upload', user.id, TRACE_UPLOAD_RATE_LIMIT)
            const { hash } = await context.params
            if (classifyChartHash(hash) === 'unknown') throw createAppError(ERROR_CODE.IR_CHART_NOT_FOUND, { hash })
            const input = await parseLimitedJsonBody(request, traceUploadSchema, TRACE_MAX_BYTES)
            const result = await deps.traceService.upload({ input, pathHash: hash, userId: user.id })
            if (result.outcome === TRACE_UPLOAD_OUTCOME.SCORE_NOT_FOUND) throw createAppError(ERROR_CODE.IR_SCORE_NOT_FOUND, { field: 'score_id' })
            if (result.outcome === TRACE_UPLOAD_OUTCOME.MISMATCH) {
                throw createAppError(ERROR_CODE.IR_TRACE_MISMATCH, { field: result.field, reason: result.reason })
            }
            if (result.outcome === TRACE_UPLOAD_OUTCOME.EXISTING) return irRaw({ id: result.id, storage: result.storage }, OK_STATUS)
            return irRaw({ id: result.id, analysis: result.analysis, storage: result.storage }, CREATED_STATUS)
        }),

        mine: withUser<unknown>(async (_request, _context, user, deps) => privateJson(successResponse(await deps.traceService.usageOf(user.id)))),

        adminUsage: withAdminToken<unknown>(async (_request, _context, _user, deps) =>
            privateJson(successResponse(await deps.traceService.adminUsage())),
        ),

        sharingView: withUser<unknown>(async (_request, _context, user, deps) =>
            privateJson(successResponse(await deps.traceService.shareSettingOf(user.id))),
        ),

        sharingPut: withUser<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('trace-share', user.id, TRACE_SHARE_WRITE_RATE_LIMIT)
            const input = await parseLimitedJsonBody(request, aiShareSettingPutSchema, AI_LIMIT.WRITE_MAX_BYTES)
            return privateJson(successResponse(await deps.traceService.putShareSetting({ userId: user.id, rivalSegments: input.rival_segments })))
        }),

        adminRecompute: withAdminToken<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('trace-recompute', user.id, TRACE_RECOMPUTE_RATE_LIMIT)
            const input = await parseLimitedJsonBody(request, traceRecomputeSchema, AI_LIMIT.WRITE_MAX_BYTES)
            return privateJson(successResponse(await deps.traceService.recomputeStale(input)))
        }),
    }
}
