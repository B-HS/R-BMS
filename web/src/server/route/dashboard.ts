import { NextResponse, connection } from 'next/server'
import { compose } from '@server/compose'
import { dashboardCalendarQuerySchema, dashboardReplaysQuerySchema, dashboardRivalsQuerySchema } from '@server/dto/dashboard'
import { tableIdParamSchema } from '@server/dto/table-snapshot'
import { successResponse } from '@server/lib/api-response'
import type { AuthUser } from '@server/lib/auth-context'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { parseSearchParams } from '@server/lib/parse-request'
import { withErrorHandling } from '@server/lib/with-error-handling'
import { RATE_LIMIT, enforceUserRateLimit } from '@server/lib/with-rate-limit'
import { withSession } from '@server/lib/with-session'
import type { AuthService } from '@server/service/domain/auth/auth.service'
import {
    readDashboardCalendar,
    readDashboardCourses,
    readDashboardOverview,
    readDashboardReplays,
    readDashboardRivals,
    readDashboardTableProgress,
} from '@server/service/dashboard-reads'

export type DashboardReads = {
    overview: typeof readDashboardOverview
    calendar: typeof readDashboardCalendar
    tableProgress: typeof readDashboardTableProgress
    courses: typeof readDashboardCourses
    rivals: typeof readDashboardRivals
    replays: typeof readDashboardReplays
}

export type DashboardRouteDeps = {
    authService: Pick<AuthService, 'resolveSessionUser'>
    reads: DashboardReads
}

type TableContext = { params: Promise<{ tableId: string }> }

const PRIVATE_HEADERS = { 'Cache-Control': 'private, no-store' }
const RATE_LIMIT_SCOPE = 'dashboard'

const resolveDashboardRouteDeps = (): DashboardRouteDeps => ({
    authService: compose().authService,
    reads: {
        overview: readDashboardOverview,
        calendar: readDashboardCalendar,
        tableProgress: readDashboardTableProgress,
        courses: readDashboardCourses,
        rivals: readDashboardRivals,
        replays: readDashboardReplays,
    },
})

const privateJson = (data: unknown) => NextResponse.json(successResponse(data), { headers: PRIVATE_HEADERS })

const parseTableId = async (context: TableContext) => {
    const { tableId } = await context.params
    const parsed = tableIdParamSchema.safeParse(tableId)
    if (!parsed.success) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND)
    return parsed.data
}

/**
 * The signed-in player's dashboard figures (plan 8.3). Every GET answers `private, no-store` and shares one per-player budget of
 * `RATE_LIMIT.DASHBOARD` a minute. `waitForRequest` keeps the handlers out of build-time prerendering; tests pass a no-op because
 * `connection()` needs a Next request scope.
 */
export const createDashboardRoutes = (
    resolveDeps: () => DashboardRouteDeps = resolveDashboardRouteDeps,
    waitForRequest: () => Promise<void> = connection,
) => {
    const withUser = <TContext>(handler: (request: Request, context: TContext, user: AuthUser, deps: DashboardRouteDeps) => Promise<Response>) =>
        withErrorHandling(async (request: Request, context: TContext) => {
            await waitForRequest()
            const deps = resolveDeps()
            return withSession({ resolveSessionUser: deps.authService.resolveSessionUser })(async (authedRequest, authedContext: TContext, user) => {
                enforceUserRateLimit(RATE_LIMIT_SCOPE, user.id, RATE_LIMIT.DASHBOARD)
                return handler(authedRequest, authedContext, user, deps)
            })(request, context)
        })

    return {
        overview: withUser<unknown>(async (_request, _context, user, deps) => privateJson(await deps.reads.overview(user))),

        calendar: withUser<unknown>(async (request, _context, user, deps) =>
            privateJson(await deps.reads.calendar(user, parseSearchParams(request, dashboardCalendarQuerySchema))),
        ),

        tableProgress: withUser<TableContext>(async (_request, context, user, deps) => {
            const tableId = await parseTableId(context)
            const progress = await deps.reads.tableProgress(user, tableId)
            if (!progress) throw createAppError(ERROR_CODE.IR_TABLE_NOT_FOUND, { tableId })
            return privateJson(progress)
        }),

        courses: withUser<unknown>(async (_request, _context, user, deps) => privateJson(await deps.reads.courses(user))),

        rivals: withUser<unknown>(async (request, _context, user, deps) =>
            privateJson(await deps.reads.rivals(user, parseSearchParams(request, dashboardRivalsQuerySchema))),
        ),

        replays: withUser<unknown>(async (request, _context, user, deps) =>
            privateJson(await deps.reads.replays(user, parseSearchParams(request, dashboardReplaysQuerySchema))),
        ),
    }
}
