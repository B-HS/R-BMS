import { and, eq } from 'drizzle-orm'
import type { Database } from '@server/db'
import { tableSnapshot } from '@server/db/schema'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import type { CourseCatalogService } from '@server/service/domain/course-catalog/course-catalog.service'
import type { CatalogSyncPlan } from '@server/service/domain/course-catalog/course-catalog.type'
import { tableCatalog } from '@server/service/domain/course-catalog/table-courses'
import { REMOTE_TABLES } from '@server/service/domain/table-refresh/table-sources'
import {
    createTableRefreshService,
    type TableCourseSync,
    type TableCourseSyncInput,
    type TableDanAward,
    type TableSnapshotStoreDb,
} from '@server/service/domain/table-refresh/table-refresh.service'
import { fetchLimited } from '@server/service/shared/http/fetch-limited'
import { gunzipBytes, gzipBytes } from '@server/service/shared/storage/gzip-json'

const versionSelection = {
    tableId: tableSnapshot.tableId,
    headerSha256: tableSnapshot.headerSha256,
    dataSha256: tableSnapshot.dataSha256,
    fetchedAt: tableSnapshot.fetchedAt,
    checkedAt: tableSnapshot.checkedAt,
    lastOutcome: tableSnapshot.lastOutcome,
}

/** The stored copies of the remote tables: the bytes of each copy are kept gzip-compressed exactly as they were downloaded. */
export const createTableSnapshotStore = (db: Pick<Database, 'select' | 'insert' | 'update'>): TableSnapshotStoreDb => ({
    listVersions: async () => db.select(versionSelection).from(tableSnapshot),

    loadSnapshot: async (tableId) => {
        const [row] = await db.select().from(tableSnapshot).where(eq(tableSnapshot.tableId, tableId)).limit(1)
        if (!row) return null
        const { headerGz, dataGz, ...rest } = row
        return { ...rest, headerBytes: await gunzipBytes(headerGz), dataBytes: await gunzipBytes(dataGz) }
    },

    saveSnapshot: async (row, previousDataSha256) => {
        const { headerBytes, dataBytes, ...rest } = row
        const values = { ...rest, headerGz: await gzipBytes(headerBytes), dataGz: await gzipBytes(dataBytes) }
        if (previousDataSha256 !== null) {
            const { tableId, ...changes } = values
            const [result] = await db
                .update(tableSnapshot)
                .set(changes)
                .where(and(eq(tableSnapshot.tableId, tableId), eq(tableSnapshot.dataSha256, previousDataSha256)))
            return result.affectedRows > 0
        }
        try {
            await db.insert(tableSnapshot).values(values)
            return true
        } catch (error) {
            if (hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])) return false
            throw error
        }
    },

    markChecked: async ({ tableId, checkedAt, outcome }) => {
        await db.update(tableSnapshot).set({ checkedAt, lastOutcome: outcome }).where(eq(tableSnapshot.tableId, tableId))
    },
})

const groupPlanOf = (plan: CatalogSyncPlan, groupId: string) => {
    const group = plan.groups.find((candidate) => candidate.id === groupId)
    if (!group) throw createAppError(ERROR_CODE.INTERNAL_ERROR, { reason: 'table_course_group_missing' })
    return group
}

/** Registers the courses a table header lists through the course catalog: one table is one group whose id is the table id. */
export const tableCourseSyncOf = (catalog: Pick<CourseCatalogService, 'planSync' | 'applySync'>): TableCourseSync => {
    const definitionOf = (input: TableCourseSyncInput) => tableCatalog(input).definition
    return {
        planSync: async (input) => groupPlanOf(await catalog.planSync(definitionOf(input)), input.table.id),
        applySync: async (input) => groupPlanOf(await catalog.applySync(definitionOf(input)), input.table.id),
    }
}

export const composeTableRefresh = (
    db: Pick<Database, 'select' | 'insert' | 'update'>,
    deps: { courses: TableCourseSync | null; dan: TableDanAward | null },
) => {
    const tableRefreshService = createTableRefreshService({
        tables: REMOTE_TABLES,
        fetchBytes: fetchLimited,
        store: createTableSnapshotStore(db),
        courses: deps.courses,
        dan: deps.dan,
        now: () => Date.now(),
    })

    return { tableRefreshService }
}
