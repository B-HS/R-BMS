import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { TRACE_MAX_BYTES, TRACE_UPLOAD_RATE_LIMIT, traceUploadSchema } from '@server/dto/trace'
import { irRaw, successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { guardBodySize, parseLimitedJsonBody } from '@server/lib/parse-request'
import { parseBearerToken } from '@server/lib/token'
import { withAdminBearer } from '@server/lib/with-admin-bearer'
import { withApiToken } from '@server/lib/with-api-token'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withSession } from '@server/lib/with-session'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import { classifyChartHash } from '@server/service/domain/chart/chart.service'
import { TRACE_UPLOAD_OUTCOME, type TraceService } from '@server/service/domain/trace/trace.service'

export type TraceRouteDeps = {
    authService: Pick<AuthService, 'resolveApiTokenUser' | 'resolveSessionUser'>
    traceService: Pick<TraceService, 'upload' | 'usageOf' | 'adminUsage'>
}

type HashContext = { params: Promise<{ hash: string }> }

type TraceHandler<TContext> = (request: Request, context: TContext, user: AuthUser, deps: TraceRouteDeps) => Promise<Response>

const CREATED_STATUS = 201
const OK_STATUS = 200
const PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

const privateJson = (body: unknown) => NextResponse.json(body, { headers: PRIVATE_HEADERS })

/**
 * The play trace routes. The client uploads with its API token (`POST /charts/{hash}/traces`); a trace is never read back by any
 * route. The account's own quota view (session) and the admin usage (admin API token) answer counts only.
 */
export const createTraceRoutes = (resolveDeps: () => TraceRouteDeps = compose, waitForRequest: () => Promise<void> = connection) => {
    const withDeps =
        <TContext>(run: (request: Request, context: TContext, deps: TraceRouteDeps) => Promise<Response>) =>
        async (request: Request, context: TContext) => {
            await waitForRequest()
            return run(request, context, resolveDeps())
        }

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
    }
}
