import { NextResponse } from 'next/server'
import { compose } from '@server/compose'
import { successResponse } from '@server/lib/api-response'
import { getEnv } from '@server/lib/env'
import { withCronSecret } from '@server/lib/with-cron-secret'
import { withErrorHandling } from '@server/lib/with-error-handling'
import type { TableRefreshService } from '@server/service/domain/table-refresh/table-refresh.service'
import { revalidateOnTableRefresh } from '@server/service/read-cache'

export type TableRefreshCronDeps = {
    tableRefreshService: Pick<TableRefreshService, 'refreshAll'>
    revalidate: { onTableRefresh: typeof revalidateOnTableRefresh }
}

const PRIVATE_CACHE = 'private, no-store'

const resolveTableRefreshCronDeps = (): TableRefreshCronDeps => ({
    tableRefreshService: compose().tableRefreshService,
    revalidate: { onTableRefresh: revalidateOnTableRefresh },
})

/**
 * The daily copy of the remote difficulty tables, called by Vercel Cron with `Authorization: Bearer <CRON_SECRET>`. The answer lists what
 * happened to each table, as numbers and fixed reason codes only, and is never cached. What the refresh changed is dropped from the read
 * caches (course catalog, profiles of players who were given dans) before the answer is sent.
 */
export const createTableRefreshCronRoute = (
    resolveDeps: () => TableRefreshCronDeps = resolveTableRefreshCronDeps,
    readSecret: () => string | undefined = () => getEnv().CRON_SECRET,
) =>
    withErrorHandling(
        withCronSecret({ readSecret })(async () => {
            const { tableRefreshService, revalidate } = resolveDeps()
            const tables = await tableRefreshService.refreshAll({ allowLargeChange: false, dryRun: false })
            revalidate.onTableRefresh(tables)
            return NextResponse.json(successResponse({ tables }), { headers: { 'Cache-Control': PRIVATE_CACHE } })
        }),
    )
