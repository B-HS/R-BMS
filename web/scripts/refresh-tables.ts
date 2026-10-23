import { composeCourseCatalog } from '../src/server/compose/course-catalog'
import { composeDan } from '../src/server/compose/dan'
import { composeTableRefresh, tableCourseSyncOf } from '../src/server/compose/table-refresh'
import { summarizeError } from '../src/server/lib/error-summary'
import type { TableRefreshReport } from '../src/server/service/domain/table-refresh/table-refresh.service'
import { TABLE_REFRESH_OUTCOME } from '../src/server/service/domain/table-refresh/table-refresh.validate'
import { createScriptDb, DATABASE_NOT_SET_MESSAGE } from './lib/script-db'

const EXIT_CODE = { OK: 0, FAILED: 1, USAGE: 2 } as const

export const REFRESH_MODE = { COUNT: 'count', APPLY_LARGE: 'apply-large' } as const
export type RefreshMode = (typeof REFRESH_MODE)[keyof typeof REFRESH_MODE]

const USAGE = `bun scripts/refresh-tables.ts <${REFRESH_MODE.COUNT}|${REFRESH_MODE.APPLY_LARGE}>
  ${REFRESH_MODE.COUNT}: downloads the remote tables and checks them against the stored copies; writes nothing
  ${REFRESH_MODE.APPLY_LARGE}: stores the downloaded tables even when they shrank or changed past the daily limits`

export const parseRefreshMode = (argument: string | undefined): RefreshMode | null =>
    Object.values(REFRESH_MODE).find((mode) => mode === argument) ?? null

/** One log line per table: the table id, fixed outcome and reason codes, and counts. Nothing taken from the downloaded tables is printed. */
export const formatReport = (report: TableRefreshReport) =>
    [
        report.tableId,
        report.outcome,
        report.reason ?? '-',
        `entries=${report.entries ?? '-'}`,
        `added=${report.added ?? '-'}`,
        `removed=${report.removed ?? '-'}`,
        `releveled=${report.releveled ?? '-'}`,
    ].join(' ')

/** A table that could not be downloaded always fails the run. A refusal fails it only for `apply-large`, where the size limits are already lifted. */
export const exitCodeFor = (mode: RefreshMode, reports: readonly TableRefreshReport[]) => {
    const isFailure = (report: TableRefreshReport) =>
        report.outcome === TABLE_REFRESH_OUTCOME.FAILED || (mode === REFRESH_MODE.APPLY_LARGE && report.outcome === TABLE_REFRESH_OUTCOME.REJECTED)
    return reports.some(isFailure) ? EXIT_CODE.FAILED : EXIT_CODE.OK
}

const run = async (mode: RefreshMode) => {
    const scriptDb = createScriptDb()
    if (!scriptDb) {
        console.error(DATABASE_NOT_SET_MESSAGE)
        return EXIT_CODE.FAILED
    }
    const { db, close } = scriptDb
    try {
        const { courseCatalogService } = composeCourseCatalog(db)
        const { danService } = composeDan(db)
        const { tableRefreshService } = composeTableRefresh(db, { courses: tableCourseSyncOf(courseCatalogService), dan: danService })
        const reports = await tableRefreshService.refreshAll({
            allowLargeChange: mode === REFRESH_MODE.APPLY_LARGE,
            dryRun: mode === REFRESH_MODE.COUNT,
        })
        reports.forEach((report) => console.log(formatReport(report)))
        return exitCodeFor(mode, reports)
    } finally {
        await close()
    }
}

const main = async () => {
    const mode = parseRefreshMode(process.argv[2])
    if (!mode) {
        console.error(USAGE)
        process.exit(EXIT_CODE.USAGE)
    }
    process.exit(await run(mode))
}

if (import.meta.main) {
    main().catch((error: unknown) => {
        console.error('failed', summarizeError(error))
        process.exit(EXIT_CODE.FAILED)
    })
}
