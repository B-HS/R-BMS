import { unstable_rethrow } from 'next/navigation'
import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { clientBuildSchema } from '@server/dto/admin'
import {
    CHART_PATTERN_MAX_BYTES,
    CHART_PATTERN_RECOMPUTE_MAX_BYTES,
    CHART_PATTERN_RECOMPUTE_RATE_LIMIT,
    CHART_PATTERN_UPLOAD_OUTCOME,
    CHART_PATTERN_WRITE_RATE_LIMIT,
    chartPatternDocumentSchema,
    chartPatternMd5ParamSchema,
    chartPatternRecomputeSchema,
} from '@server/dto/chart-pattern'
import { irRaw, successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError, isAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { summarizeError } from '@server/lib/error-summary'
import { parseJsonBody, parseLimitedJsonBody, parseQueryObject } from '@server/lib/parse-request'
import { withAdmin } from '@server/lib/with-session'
import { withAdminBearer } from '@server/lib/with-admin-bearer'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { enforceUserRateLimit } from '@server/lib/with-rate-limit'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import type { ChartPatternService } from '@server/service/domain/chart-pattern/chart-pattern.service'
import { readClientBuilds } from '@server/service/public-reads'
import { revalidateOnBuildUpsert } from '@server/service/read-cache'

const CREATED_STATUS = 201
const OK_STATUS = 200

export const createAdminBuildListRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService } = compose()
        return withAdmin({ resolveSessionUser: authService.resolveSessionUser })(async () => irRaw(successResponse(await readClientBuilds())))(
            request,
            context,
        )
    })

export const createAdminBuildUpsertRoute = () =>
    withErrorHandling(async (request: Request, context: unknown) => {
        const { authService, buildService } = compose()
        return withAdmin({ resolveSessionUser: authService.resolveSessionUser })(async (authedRequest) => {
            const input = await parseJsonBody(authedRequest, clientBuildSchema)
            const build = await buildService.upsert(input)
            revalidateOnBuildUpsert()
            return irRaw(successResponse(build), CREATED_STATUS)
        })(request, context)
    })

type Md5Context = { params: Promise<{ md5: string }> }

export type AdminChartPatternRouteDeps = {
    authService: Pick<AuthService, 'resolveApiTokenUser'>
    chartPatternService: Pick<ChartPatternService, 'upload' | 'coverage' | 'recomputeStale'>
}

type AdminTokenHandler<TContext> = (request: Request, context: TContext, user: AuthUser, deps: AdminChartPatternRouteDeps) => Promise<Response>

const ADMIN_PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

const adminPrivateJson = (body: unknown, status = 200) => NextResponse.json(body, { status, headers: ADMIN_PRIVATE_HEADERS })

/**
 * Logs only the class and driver code of an unexpected error: a database error message carries the query and its bound values,
 * which for these routes is a chart's note layout.
 */
const withSummarizedErrors =
    <TContext>(run: (request: Request, context: TContext) => Promise<Response>) =>
    async (request: Request, context: TContext) => {
        try {
            return await run(request, context)
        } catch (error) {
            if (isAppError(error)) throw error
            unstable_rethrow(error)
            const summary = summarizeError(error)
            console.error('chart pattern route failed', summary)
            throw createAppError(ERROR_CODE.INTERNAL_ERROR, summary)
        }
    }

/**
 * The chart-pattern admin routes: an API token of an admin user only (`withAdminBearer`), never a session. There is no route that
 * reads a pattern back; the coverage answers counts and table md5s, and the recompute answers counts and the md5 to continue from.
 */
export const createAdminChartPatternRoutes = (
    resolveDeps: () => AdminChartPatternRouteDeps = compose,
    waitForRequest: () => Promise<void> = connection,
) => {
    const withAdminToken = <TContext>(handler: AdminTokenHandler<TContext>) =>
        withErrorHandling(
            withSummarizedErrors(async (request: Request, context: TContext) => {
                await waitForRequest()
                const deps = resolveDeps()
                return withAdminBearer({ resolveApiTokenUser: deps.authService.resolveApiTokenUser })(
                    (authedRequest, authedContext: TContext, user) => handler(authedRequest, authedContext, user, deps),
                )(request, context)
            }),
        )

    return {
        put: withAdminToken<Md5Context>(async (request, context, user, deps) => {
            enforceUserRateLimit('chart-pattern-write', user.id, CHART_PATTERN_WRITE_RATE_LIMIT)
            const { md5 } = parseQueryObject(await context.params, chartPatternMd5ParamSchema)
            const document = await parseLimitedJsonBody(request, chartPatternDocumentSchema, CHART_PATTERN_MAX_BYTES)
            if (document.md5 !== md5) throw createAppError(ERROR_CODE.IR_CHART_PATTERN_INVALID, { field: 'md5', reason: 'md5_mismatch' })
            const result = await deps.chartPatternService.upload({ document, uploadedBy: user.id })
            return adminPrivateJson(successResponse(result), result.outcome === CHART_PATTERN_UPLOAD_OUTCOME.CREATED ? CREATED_STATUS : OK_STATUS)
        }),

        coverage: withAdminToken<unknown>(async (_request, _context, _user, deps) =>
            adminPrivateJson(successResponse(await deps.chartPatternService.coverage())),
        ),

        recompute: withAdminToken<unknown>(async (request, _context, user, deps) => {
            enforceUserRateLimit('chart-pattern-recompute', user.id, CHART_PATTERN_RECOMPUTE_RATE_LIMIT)
            const input = await parseLimitedJsonBody(request, chartPatternRecomputeSchema, CHART_PATTERN_RECOMPUTE_MAX_BYTES)
            return adminPrivateJson(successResponse(await deps.chartPatternService.recomputeStale(input)))
        }),
    }
}
