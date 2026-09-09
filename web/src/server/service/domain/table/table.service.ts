import type { CourseMetaInput } from '@server/dto/course'
import type { TableDataInput } from '@server/dto/table'

export type TableRow = {
    id: string
    name: string
    url: string | null
    extra: unknown
}

export type TableFolderRow = {
    id: string
    name: string
    position: number
}

export type TableChartRow = {
    folderId: string
    chartSha256: string
    md5: string | null
}

export type TableServiceDb = {
    listTables: () => Promise<TableRow[]>
    findTable: (id: string) => Promise<TableRow | null>
    listFolders: (tableIds: string[]) => Promise<(TableFolderRow & { tableId: string })[]>
    listFolderCharts: (folderIds: string[]) => Promise<TableChartRow[]>
    listTableCourseHashes: (tableIds: string[]) => Promise<{ tableId: string; courseHash: string }[]>
    upsertTable: (row: { id: string; name: string; url: string | null; extra: Record<string, unknown> }) => Promise<void>
    replaceFolders: (tableId: string, folders: { id: string; name: string; position: number; charts: string[] }[]) => Promise<void>
    replaceTableCourses: (tableId: string, courseHashes: string[]) => Promise<void>
}

export type TableServiceDeps = {
    db: TableServiceDb
    newId: (prefix: string) => string
    loadCourses: (courseHashes: string[]) => Promise<CourseMetaInput[]>
}

export const folderIdFor = (tableId: string, position: number) => `${tableId}#${position}`

export const assembleTable = (params: {
    table: TableRow
    folders: TableFolderRow[]
    charts: TableChartRow[]
    courses: CourseMetaInput[]
}): TableDataInput => ({
    id: params.table.id,
    name: params.table.name,
    url: params.table.url,
    folders: params.folders
        .slice()
        .sort((left, right) => left.position - right.position)
        .map((folder) => ({
            name: folder.name,
            charts: params.charts
                .filter((chart) => chart.folderId === folder.id)
                .map((chart) => ({ md5: chart.md5 ?? '', sha256: chart.chartSha256 })),
        })),
    courses: params.courses,
    extra: (params.table.extra as Record<string, unknown>) ?? {},
})

export const createTableService = (deps: TableServiceDeps) => {
    const assembleMany = async (tables: TableRow[]) => {
        if (tables.length === 0) return []
        const tableIds = tables.map((table) => table.id)
        const folders = await deps.db.listFolders(tableIds)
        const charts = folders.length > 0 ? await deps.db.listFolderCharts(folders.map((folder) => folder.id)) : []
        const courseLinks = await deps.db.listTableCourseHashes(tableIds)
        const courses = courseLinks.length > 0 ? await deps.loadCourses(courseLinks.map((link) => link.courseHash)) : []
        return tables.map((table) =>
            assembleTable({
                table,
                folders: folders.filter((folder) => folder.tableId === table.id),
                charts,
                courses: courses.filter((course) => courseLinks.some((link) => link.tableId === table.id && link.courseHash === course.course_hash)),
            }),
        )
    }

    return {
        list: async () => assembleMany(await deps.db.listTables()),

        getById: async (id: string) => {
            const table = await deps.db.findTable(id)
            if (!table) return null
            const [assembled] = await assembleMany([table])
            return assembled ?? null
        },

        upsert: async (input: TableDataInput) => {
            await deps.db.upsertTable({ id: input.id, name: input.name, url: input.url ?? null, extra: input.extra })
            await deps.db.replaceFolders(
                input.id,
                input.folders.map((folder, position) => ({
                    id: folderIdFor(input.id, position),
                    name: folder.name,
                    position,
                    charts: folder.charts.map((chart) => chart.sha256).filter((sha256) => sha256.length > 0),
                })),
            )
            await deps.db.replaceTableCourses(
                input.id,
                input.courses.map((course) => course.course_hash),
            )
            const table = await deps.db.findTable(input.id)
            if (!table) return null
            const [assembled] = await assembleMany([table])
            return assembled ?? null
        },
    }
}

export type TableService = ReturnType<typeof createTableService>
