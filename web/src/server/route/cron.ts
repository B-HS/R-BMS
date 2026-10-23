import { NextResponse } from 'next/server'
import { compose } from '@server/compose'
import { successResponse } from '@server/lib/api-response'
import { getEnv } from '@server/lib/env'
import { withCronSecret } from '@server/lib/with-cron-secret'
import { withErrorHandling } from '@server/lib/with-error-handling'
import type { TableRefreshService } from '@server/service/domain/table-refresh/table-refresh.service'

export type TableRefreshCronDeps = {
    tableRefreshService: Pick<TableRefreshService, 'refreshAll'>
}

const PRIVATE_CACHE = 'private, no-store'

/**
 * The daily copy of the remote difficulty tables, called by Vercel Cron with `Authorization: Bearer <CRON_SECRET>`. The answer lists what
 * happened to each table, as numbers and fixed reason codes only, and is never cached.
 */
export const createTableRefreshCronRoute = (
    resolveDeps: () => TableRefreshCronDeps = compose,
    readSecret: () => string | undefined = () => getEnv().CRON_SECRET,
) =>
    withErrorHandling(
        withCronSecret({ readSecret })(async () => {
            const tables = await resolveDeps().tableRefreshService.refreshAll({ allowLargeChange: false, dryRun: false })
            return NextResponse.json(successResponse({ tables }), { headers: { 'Cache-Control': PRIVATE_CACHE } })
        }),
    )
