import { asc, eq, inArray } from 'drizzle-orm'
import type { Database } from '@server/db'
import { chart, difficultyTable, tableChart, tableCourse, tableFolder } from '@server/db/schema'
import { newId } from '@server/lib/token'
import { createTableService } from '@server/service/domain/table/table.service'
import type { CourseService } from '@server/service/domain/course/course.service'

export const composeTable = (db: Database, courseService: CourseService) => {
    const tableService = createTableService({
        newId,
        loadCourses: async (courseHashes) => {
            const metas = await Promise.all(courseHashes.map((courseHash) => courseService.getMeta(courseHash)))
            return metas.filter((meta) => meta !== null)
        },
        db: {
            listTables: async () =>
                db
                    .select({ id: difficultyTable.id, name: difficultyTable.name, url: difficultyTable.url, extra: difficultyTable.extra })
                    .from(difficultyTable),
            findTable: async (id) => {
                const [row] = await db
                    .select({ id: difficultyTable.id, name: difficultyTable.name, url: difficultyTable.url, extra: difficultyTable.extra })
                    .from(difficultyTable)
                    .where(eq(difficultyTable.id, id))
                    .limit(1)
                return row ?? null
            },
            listFolders: async (tableIds) =>
                db
                    .select({ id: tableFolder.id, tableId: tableFolder.tableId, name: tableFolder.name, position: tableFolder.position })
                    .from(tableFolder)
                    .where(inArray(tableFolder.tableId, tableIds))
                    .orderBy(asc(tableFolder.position)),
            listFolderCharts: async (folderIds) =>
                db
                    .select({ folderId: tableChart.folderId, chartSha256: tableChart.chartSha256, md5: chart.md5 })
                    .from(tableChart)
                    .leftJoin(chart, eq(chart.sha256, tableChart.chartSha256))
                    .where(inArray(tableChart.folderId, folderIds)),
            listTableCourseHashes: async (tableIds) =>
                db
                    .select({ tableId: tableCourse.tableId, courseHash: tableCourse.courseHash })
                    .from(tableCourse)
                    .where(inArray(tableCourse.tableId, tableIds)),
            upsertTable: async (row) => {
                await db
                    .insert(difficultyTable)
                    .values(row)
                    .onDuplicateKeyUpdate({ set: { name: row.name, url: row.url, extra: row.extra } })
            },
            replaceFolders: async (tableId, folders) =>
                db.transaction(async (tx) => {
                    const existing = await tx.select({ id: tableFolder.id }).from(tableFolder).where(eq(tableFolder.tableId, tableId))
                    if (existing.length > 0) {
                        await tx.delete(tableChart).where(
                            inArray(
                                tableChart.folderId,
                                existing.map((folder) => folder.id),
                            ),
                        )
                    }
                    await tx.delete(tableFolder).where(eq(tableFolder.tableId, tableId))
                    if (folders.length === 0) return
                    await tx
                        .insert(tableFolder)
                        .values(folders.map((folder) => ({ id: folder.id, tableId, name: folder.name, position: folder.position })))
                    const charts = folders.flatMap((folder) => folder.charts.map((chartSha256) => ({ folderId: folder.id, chartSha256 })))
                    if (charts.length > 0) await tx.insert(tableChart).values(charts)
                }),
            replaceTableCourses: async (tableId, courseHashes) =>
                db.transaction(async (tx) => {
                    await tx.delete(tableCourse).where(eq(tableCourse.tableId, tableId))
                    if (courseHashes.length === 0) return
                    await tx.insert(tableCourse).values(courseHashes.map((courseHash) => ({ tableId, courseHash })))
                }),
        },
    })

    return { tableService }
}
