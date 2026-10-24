import { createHash } from 'node:crypto'
import type { RemoteEntry, RemoteHeader } from '@server/dto/table-refresh'
import type { SnapshotEntry } from '@server/dto/table-snapshot'
import { summarizeError } from '@server/lib/error-summary'
import type { FetchBytes } from '@server/service/shared/http/fetch-limited'
import type { CatalogGroupPlan } from '@server/service/domain/course-catalog/course-catalog.type'
import type { tableCatalog } from '@server/service/domain/course-catalog/table-courses'
import {
    TABLE_DATA_MAX_BYTES,
    TABLE_FETCH_TIMEOUT_MS,
    TABLE_FETCH_USER_AGENT,
    TABLE_HEADER_MAX_BYTES,
    type TableSourceDefinition,
} from '@server/service/domain/table-refresh/table-sources'
import {
    TABLE_REFRESH_OUTCOME,
    TABLE_REFRESH_REASON,
    diffEntries,
    judgeChange,
    parseJsonBytes,
    validateRemoteEntries,
    validateRemoteHeader,
    type EntryDiff,
    type TableRefreshOutcome,
    type TableRefreshReason,
} from '@server/service/domain/table-refresh/table-refresh.validate'

const HTTP_OK = 200

export type RemoteSnapshotVersion = {
    tableId: string
    headerSha256: string
    dataSha256: string
    fetchedAt: number
    checkedAt: number
    lastOutcome: string
}

export type RemoteSnapshotRow = RemoteSnapshotVersion & {
    headerUrl: string
    dataUrl: string
    headerBytes: Uint8Array
    dataBytes: Uint8Array
    entryCount: number
    levelCount: number
}

export type TableSnapshotStoreDb = {
    listVersions: () => Promise<RemoteSnapshotVersion[]>
    loadSnapshot: (tableId: string) => Promise<RemoteSnapshotRow | null>
    saveSnapshot: (row: RemoteSnapshotRow, previousDataSha256: string | null) => Promise<boolean>
    markChecked: (params: { tableId: string; checkedAt: number; outcome: string }) => Promise<void>
}

/** What the courses of one table are registered from: the arguments of `tableCatalog`. */
export type TableCourseSyncInput = Parameters<typeof tableCatalog>[0]

export type TableCoursePlan = CatalogGroupPlan

export type TableCourseSync = {
    planSync: (input: TableCourseSyncInput) => Promise<TableCoursePlan>
    applySync: (input: TableCourseSyncInput) => Promise<TableCoursePlan>
}

export type TableDanAward = {
    awardForCourses: (params: { courseHashes: string[]; dryRun: boolean }) => Promise<{ scanned: number; awarded: number; awardedAssisted: number }>
}

export type TableRefreshDeps = {
    tables: readonly TableSourceDefinition[]
    fetchBytes: FetchBytes
    store: TableSnapshotStoreDb
    courses: TableCourseSync | null
    dan: TableDanAward | null
    now: () => number
    logger?: Pick<Console, 'info' | 'error'>
}

export type TableRefreshOptions = { allowLargeChange: boolean; dryRun: boolean }

export type TableRefreshReport = {
    tableId: string
    outcome: TableRefreshOutcome
    reason: TableRefreshReason | null
    entries: number | null
    added: number | null
    removed: number | null
    releveled: number | null
    courses: TableCoursePlan | null
    awarded: number | null
}

type Rejection = { ok: false; outcome: typeof TABLE_REFRESH_OUTCOME.FAILED | typeof TABLE_REFRESH_OUTCOME.REJECTED; reason: TableRefreshReason }

type Downloaded = { ok: true; bytes: Uint8Array; value: unknown }

type FetchedTable = {
    ok: true
    header: RemoteHeader
    dataUrl: string
    headerBytes: Uint8Array
    dataBytes: Uint8Array
    headerSha256: string
    dataSha256: string
    entries: RemoteEntry[]
}

type Settled = { report: TableRefreshReport; courseInput: TableCourseSyncInput | null }

const sha256Hex = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex')

const failed = (reason: TableRefreshReason): Rejection => ({ ok: false, outcome: TABLE_REFRESH_OUTCOME.FAILED, reason })

const rejected = (reason: TableRefreshReason): Rejection => ({ ok: false, outcome: TABLE_REFRESH_OUTCOME.REJECTED, reason })

const emptyReport = (tableId: string, outcome: TableRefreshOutcome, reason: TableRefreshReason | null = null): TableRefreshReport => ({
    tableId,
    outcome,
    reason,
    entries: null,
    added: null,
    removed: null,
    releveled: null,
    courses: null,
    awarded: null,
})

const withDiff = (report: TableRefreshReport, entries: number, diff: EntryDiff): TableRefreshReport => ({ ...report, entries, ...diff })

const persistedOutcome = (report: TableRefreshReport) => (report.reason === null ? report.outcome : `${report.outcome}:${report.reason}`)

const hasCourseWork = (plan: TableCoursePlan) => plan.inserted + plan.changed + plan.deactivated > 0

/** Converts a downloaded chart to the shape the table pages read. */
export const toSnapshotEntry = (entry: RemoteEntry): SnapshotEntry => ({
    md5: entry.md5,
    level: entry.level,
    title: entry.title,
    artist: entry.artist,
    url: entry.url,
    url_diff: entry.url_diff,
    name_diff: entry.name_diff,
    comment: '',
    lr2_bmsid: '',
})

export const createTableRefreshService = (deps: TableRefreshDeps) => {
    const log = deps.logger ?? console

    const download = async (url: string, maxBytes: number): Promise<Downloaded | Rejection> => {
        const fetched = await deps.fetchBytes(url, { maxBytes, timeoutMs: TABLE_FETCH_TIMEOUT_MS, userAgent: TABLE_FETCH_USER_AGENT })
        if (!fetched.ok) return failed(fetched.reason === 'too_large' ? TABLE_REFRESH_REASON.TOO_LARGE : TABLE_REFRESH_REASON.FETCH_FAILED)
        if (fetched.status !== HTTP_OK) return failed(TABLE_REFRESH_REASON.HTTP_STATUS)
        const parsed = parseJsonBytes(fetched.bytes)
        if (!parsed.ok) return rejected(parsed.reason)
        return { ok: true, bytes: fetched.bytes, value: parsed.value }
    }

    const fetchTable = async (definition: TableSourceDefinition): Promise<FetchedTable | Rejection> => {
        const headerDownload = await download(definition.headerUrl, TABLE_HEADER_MAX_BYTES)
        if (!headerDownload.ok) return headerDownload
        const header = validateRemoteHeader(definition, headerDownload.value)
        if (!header.ok) return rejected(header.reason)
        const dataDownload = await download(header.dataUrl, TABLE_DATA_MAX_BYTES)
        if (!dataDownload.ok) return dataDownload
        const entries = validateRemoteEntries(dataDownload.value)
        if (!entries.ok) return rejected(entries.reason)
        return {
            ok: true,
            header: header.header,
            dataUrl: header.dataUrl,
            headerBytes: headerDownload.bytes,
            dataBytes: dataDownload.bytes,
            headerSha256: sha256Hex(headerDownload.bytes),
            dataSha256: sha256Hex(dataDownload.bytes),
            entries: entries.entries,
        }
    }

    const loadPreviousEntries = async (previous: RemoteSnapshotVersion | undefined) => {
        if (!previous) return null
        const stored = await deps.store.loadSnapshot(previous.tableId)
        if (!stored) return null
        const parsed = parseJsonBytes(stored.dataBytes)
        if (!parsed.ok) return null
        const validated = validateRemoteEntries(parsed.value)
        return validated.ok ? validated.entries : null
    }

    const markChecked = async (report: TableRefreshReport, previous: RemoteSnapshotVersion | undefined, options: TableRefreshOptions) => {
        if (options.dryRun || !previous) return
        await deps.store.markChecked({ tableId: report.tableId, checkedAt: deps.now(), outcome: persistedOutcome(report) })
    }

    const refreshTable = async (
        definition: TableSourceDefinition,
        previous: RemoteSnapshotVersion | undefined,
        options: TableRefreshOptions,
    ): Promise<Settled> => {
        const fetched = await fetchTable(definition)
        if (!fetched.ok) {
            const report = emptyReport(definition.id, fetched.outcome, fetched.reason)
            await markChecked(report, previous, options)
            return { report, courseInput: null }
        }

        const entryCount = fetched.entries.length
        const snapshotEntries = fetched.entries.map(toSnapshotEntry)
        const courseInputAt = (fetchedAt: number): TableCourseSyncInput => ({
            table: definition,
            headerCourse: fetched.header.course ?? null,
            entries: snapshotEntries,
            headerSha256: fetched.headerSha256,
            dataSha256: fetched.dataSha256,
            dataUrl: fetched.dataUrl,
            fetchedAt,
        })

        if (previous && previous.headerSha256 === fetched.headerSha256 && previous.dataSha256 === fetched.dataSha256) {
            const report = withDiff(emptyReport(definition.id, TABLE_REFRESH_OUTCOME.UNCHANGED), entryCount, { added: 0, removed: 0, releveled: 0 })
            await markChecked(report, previous, options)
            return { report, courseInput: courseInputAt(previous.fetchedAt) }
        }

        const previousEntries = await loadPreviousEntries(previous)
        const diff = diffEntries(previousEntries ?? [], fetched.entries)
        const verdict = judgeChange({
            previousCount: previousEntries?.length ?? null,
            nextCount: entryCount,
            diff: previousEntries ? diff : null,
            allowLargeChange: options.allowLargeChange,
        })
        if (!verdict.ok) {
            const report = withDiff(emptyReport(definition.id, TABLE_REFRESH_OUTCOME.REJECTED, verdict.reason), entryCount, diff)
            await markChecked(report, previous, options)
            return { report, courseInput: null }
        }

        const report = withDiff(emptyReport(definition.id, TABLE_REFRESH_OUTCOME.SAVED), entryCount, diff)
        if (options.dryRun) return { report, courseInput: courseInputAt(deps.now()) }

        const savedAt = deps.now()
        const wasSaved = await deps.store.saveSnapshot(
            {
                tableId: definition.id,
                headerUrl: definition.headerUrl,
                dataUrl: fetched.dataUrl,
                headerSha256: fetched.headerSha256,
                dataSha256: fetched.dataSha256,
                headerBytes: fetched.headerBytes,
                dataBytes: fetched.dataBytes,
                entryCount,
                levelCount: new Set(fetched.entries.map((entry) => entry.level)).size,
                fetchedAt: savedAt,
                checkedAt: savedAt,
                lastOutcome: TABLE_REFRESH_OUTCOME.SAVED,
            },
            previous?.dataSha256 ?? null,
        )
        if (!wasSaved) return { report: withDiff(emptyReport(definition.id, TABLE_REFRESH_OUTCOME.UNCHANGED), entryCount, diff), courseInput: null }
        return { report, courseInput: courseInputAt(savedAt) }
    }

    const syncCourses = async (courseInput: TableCourseSyncInput, report: TableRefreshReport, options: TableRefreshOptions) => {
        if (!deps.courses) return report
        try {
            const plan = await deps.courses.planSync(courseInput)
            if (options.dryRun || !hasCourseWork(plan)) return { ...report, courses: plan }
            const applied = await deps.courses.applySync(courseInput)
            const award =
                deps.dan && applied.newDanHashes.length > 0
                    ? await deps.dan.awardForCourses({ courseHashes: applied.newDanHashes, dryRun: false })
                    : null
            const awarded = award === null ? null : award.awarded + award.awardedAssisted
            return { ...report, courses: applied, awarded }
        } catch (error) {
            log.error('table refresh course sync failed', { tableId: report.tableId, ...summarizeError(error) })
            return { ...report, reason: TABLE_REFRESH_REASON.COURSE_SYNC_FAILED }
        }
    }

    const logReport = (report: TableRefreshReport) => {
        const fields = {
            tableId: report.tableId,
            outcome: report.outcome,
            reason: report.reason,
            entries: report.entries,
            added: report.added,
            removed: report.removed,
            releveled: report.releveled,
        }
        const isHealthy =
            report.reason === null && (report.outcome === TABLE_REFRESH_OUTCOME.SAVED || report.outcome === TABLE_REFRESH_OUTCOME.UNCHANGED)
        if (isHealthy) log.info('table refresh', fields)
        else log.error('table refresh did not finish cleanly', fields)
    }

    const refreshOne = async (definition: TableSourceDefinition, previous: RemoteSnapshotVersion | undefined, options: TableRefreshOptions) => {
        try {
            const settled = await refreshTable(definition, previous, options)
            return settled.courseInput ? await syncCourses(settled.courseInput, settled.report, options) : settled.report
        } catch (error) {
            log.error('table refresh crashed', { tableId: definition.id, ...summarizeError(error) })
            return emptyReport(definition.id, TABLE_REFRESH_OUTCOME.FAILED, TABLE_REFRESH_REASON.UNEXPECTED_ERROR)
        }
    }

    return {
        /** Refreshes every table one after the other, so the source host sees one request at a time. One table failing never stops the next. */
        refreshAll: async (options: TableRefreshOptions) => {
            const versions = new Map((await deps.store.listVersions()).map((version) => [version.tableId, version]))
            const reports: TableRefreshReport[] = []
            for (const definition of deps.tables) {
                const report = await refreshOne(definition, versions.get(definition.id), options)
                logReport(report)
                reports.push(report)
            }
            return reports
        },
    }
}

export type TableRefreshService = ReturnType<typeof createTableRefreshService>
