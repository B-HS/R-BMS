import { clearLampFromId } from '@server/dto/common'
import type {
    SnapshotEntry,
    TableDetailOutput,
    TableEntriesQuery,
    TableEntryOutput,
    TableRecordOutput,
    TableRecordsOutput,
    TableSummaryOutput,
} from '@server/dto/table-snapshot'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'

export type BestRecordRow = {
    md5: string
    clear: number
    exScore: number
    minbp: number
    maxCombo: number
    totalNotes: number
    playedAt: number
    scratchAuto: boolean
}

export type TableSnapshotServiceDb = {
    listBestByMd5: (params: { userId: string; md5s: string[] }) => Promise<BestRecordRow[]>
}

export type TableSnapshotServiceDeps = {
    db: TableSnapshotServiceDb
    loadTables: () => Map<string, SnapshotTable>
}

const TABLES_PATH = '/tables'
const MD5_PREFIX_PATTERN = /^[0-9a-f]{6,32}$/
const ETAG_HASH_LENGTH = 32

export const standardPaths = (tableId: string) => ({
    page: `${TABLES_PATH}/${tableId}`,
    header: `${TABLES_PATH}/${tableId}/header.json`,
    data: `${TABLES_PATH}/${tableId}/data.json`,
})

export const orderedLevels = (table: SnapshotTable) => {
    const known = new Set(table.header.level_order)
    const extra = [...new Set(table.entries.map((entry) => entry.level))].filter((level) => !known.has(level))
    return [...table.header.level_order, ...extra]
}

export const toTableEntry = (entry: SnapshotEntry): TableEntryOutput => ({
    md5: entry.md5,
    level: entry.level,
    title: entry.title,
    artist: entry.artist,
    url: entry.url,
    url_diff: entry.url_diff,
    name_diff: entry.name_diff,
    comment: entry.comment,
})

export const summariseTable = (table: SnapshotTable): TableSummaryOutput => {
    const counts = new Map<string, number>()
    for (const entry of table.entries) counts.set(entry.level, (counts.get(entry.level) ?? 0) + 1)
    const levels = orderedLevels(table)
        .filter((level) => counts.has(level))
        .map((level) => ({ level, count: counts.get(level) ?? 0 }))
    return {
        id: table.id,
        name: table.header.name,
        symbol: table.header.symbol,
        entry_count: table.entries.length,
        level_count: levels.length,
        last_update: table.header.last_update,
        levels,
        standard_paths: standardPaths(table.id),
    }
}

export const groupTableByLevel = (table: SnapshotTable): TableDetailOutput => {
    const buckets = new Map<string, TableEntryOutput[]>()
    for (const entry of table.entries) {
        const bucket = buckets.get(entry.level)
        if (bucket) bucket.push(toTableEntry(entry))
        else buckets.set(entry.level, [toTableEntry(entry)])
    }
    return {
        table: summariseTable(table),
        levels: orderedLevels(table)
            .filter((level) => buckets.has(level))
            .map((level) => ({ level, entries: buckets.get(level) ?? [] })),
    }
}

export const toStandardHeader = (table: SnapshotTable, origin: string) => ({
    name: table.header.name,
    symbol: table.header.symbol,
    data_url: `${origin}${standardPaths(table.id).data}`,
    level_order: table.header.level_order,
    last_update: table.header.last_update,
})

const normaliseSearchText = (value: string) => value.normalize('NFKC').toLowerCase()

const MAX_EXTRA_MD5S = 200

export const searchMd5sInTables = (tables: Map<string, SnapshotTable>, query: string, limit = MAX_EXTRA_MD5S) => {
    const needle = normaliseSearchText(query.trim())
    if (needle === '') return []
    const found: string[] = []
    for (const table of tables.values()) {
        for (const entry of table.entries) {
            if (normaliseSearchText(entry.title).includes(needle) || normaliseSearchText(entry.artist).includes(needle)) found.push(entry.md5)
            if (found.length >= limit) return found
        }
    }
    return found
}

export const searchTableEntries = (table: SnapshotTable, query: TableEntriesQuery) => {
    const needle = query.q ? normaliseSearchText(query.q.trim()) : ''
    const matched = table.entries.filter((entry) => {
        if (query.level !== undefined && entry.level !== query.level) return false
        if (needle === '') return true
        if (MD5_PREFIX_PATTERN.test(needle) && entry.md5.startsWith(needle)) return true
        return normaliseSearchText(entry.title).includes(needle) || normaliseSearchText(entry.artist).includes(needle)
    })
    const start = (query.page - 1) * query.limit
    return { items: matched.slice(start, start + query.limit).map(toTableEntry), total: matched.length }
}

export const toTableRecord = (row: BestRecordRow): TableRecordOutput => ({
    clear: clearLampFromId(row.clear),
    ex_score: row.exScore,
    minbp: row.minbp,
    max_combo: row.maxCombo,
    total_notes: row.totalNotes,
    played_at: row.playedAt,
    scratch_auto: row.scratchAuto,
})

export const createTableSnapshotService = (deps: TableSnapshotServiceDeps) => {
    const serialisedData = new Map<string, string>()

    const list = () => [...deps.loadTables().values()].map(summariseTable)

    const find = (tableId: string) => deps.loadTables().get(tableId) ?? null

    return {
        list,

        getDetail: (tableId: string) => {
            const table = find(tableId)
            return table ? groupTableByLevel(table) : null
        },

        getStandardHeader: (tableId: string, origin: string) => {
            const table = find(tableId)
            return table ? toStandardHeader(table, origin) : null
        },

        getStandardDataText: (tableId: string) => {
            const table = find(tableId)
            if (!table) return null
            const cached = serialisedData.get(tableId)
            if (cached) return cached
            const text = JSON.stringify(table.rawEntries)
            serialisedData.set(tableId, text)
            return text
        },

        getDataEtag: (tableId: string) => {
            const table = find(tableId)
            return table ? `"${table.source.snapshot.data_file_sha256.slice(0, ETAG_HASH_LENGTH)}"` : null
        },

        searchEntries: (tableId: string, query: TableEntriesQuery) => {
            const table = find(tableId)
            return table ? searchTableEntries(table, query) : null
        },

        loadEntriesForImport: (tableId: string) => {
            const table = find(tableId)
            if (!table) return null
            return table.entries.map((entry) => ({ md5: entry.md5, sha256: '', title: entry.title, artist: entry.artist, level: entry.level }))
        },

        getRecords: async (params: { tableId: string; userId: string }): Promise<TableRecordsOutput | null> => {
            const table = find(params.tableId)
            if (!table) return null
            const rows = await deps.db.listBestByMd5({ userId: params.userId, md5s: table.entries.map((entry) => entry.md5) })
            return { table_id: table.id, records: Object.fromEntries(rows.map((row) => [row.md5, toTableRecord(row)])) }
        },
    }
}

export type TableSnapshotService = ReturnType<typeof createTableSnapshotService>
