import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { CHART_BLOCKLIST_WRITE_MAX_BYTES, CHART_BLOCKLIST_WRITE_RATE_LIMIT, chartBlocklistRegisterSchema } from '@server/dto/chart-blocklist'
import { successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { parseLimitedJsonBody } from '@server/lib/parse-request'
import { withAdminBearer } from '@server/lib/with-admin-bearer'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { enforceUserRateLimit } from '@server/lib/with-rate-limit'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import type { ChartBlocklistService } from '@server/service/domain/chart-blocklist/chart-blocklist.service'

export type ChartBlocklistRouteDeps = {
    authService: Pick<AuthService, 'resolveApiTokenUser'>
    chartBlocklistService: Pick<ChartBlocklistService, 'register' | 'listSources'>
}

type AdminHandler = (request: Request, user: AuthUser, deps: ChartBlocklistRouteDeps) => Promise<Response>

const PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }

/**
 * The chart blocklist admin routes, for an API token of an admin user only (`scripts/chart-blocklist.ts add`). `PUT` adds the charts
 * of one source no list has yet and answers `{ received, added, already_listed }`; `GET` answers each source with its chart count.
 */
export const createChartBlocklistRoutes = (
    resolveDeps: () => ChartBlocklistRouteDeps = compose,
    waitForRequest: () => Promise<void> = connection,
) => {
    const withAdminToken = (handler: AdminHandler) =>
        withErrorHandling(async (request: Request, context: unknown) => {
            await waitForRequest()
            const deps = resolveDeps()
            return withAdminBearer({ resolveApiTokenUser: deps.authService.resolveApiTokenUser })((authedRequest, _context, user) =>
                handler(authedRequest, user, deps),
            )(request, context)
        })

    return {
        put: withAdminToken(async (request, user, deps) => {
            enforceUserRateLimit('chart-blocklist-write', user.id, CHART_BLOCKLIST_WRITE_RATE_LIMIT)
            const input = await parseLimitedJsonBody(request, chartBlocklistRegisterSchema, CHART_BLOCKLIST_WRITE_MAX_BYTES)
            const result = await deps.chartBlocklistService.register({ ...input, addedBy: user.id })
            return NextResponse.json(successResponse({ received: result.received, added: result.added, already_listed: result.alreadyListed }), {
                headers: PRIVATE_HEADERS,
            })
        }),

        list: withAdminToken(async (_request, _user, deps) => {
            const sources = await deps.chartBlocklistService.listSources()
            return NextResponse.json(
                successResponse(sources.map((source) => ({ id: source.sourceId, label: source.sourceLabel, charts: source.charts }))),
                { headers: PRIVATE_HEADERS },
            )
        }),
    }
}
