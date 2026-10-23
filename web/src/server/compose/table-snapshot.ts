import { and, eq, inArray } from 'drizzle-orm'
import type { Database } from '@server/db'
import { chart, chartBest, score } from '@server/db/schema'
import { remoteEntriesSchema, remoteHeaderSchema } from '@server/dto/table-refresh'
import { REMOTE_TABLES, type TableSourceDefinition } from '@server/service/domain/table-refresh/table-sources'
import {
    toSnapshotEntry,
    type RemoteSnapshotRow,
    type RemoteSnapshotVersion,
    type TableSnapshotStoreDb,
} from '@server/service/domain/table-refresh/table-refresh.service'
import { deriveLevelOrder, parseJsonBytes } from '@server/service/domain/table-refresh/table-refresh.validate'
import { getSnapshotTables, type SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { createTableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { chartBestScope } from '@server/compose/best-scope'
import { createTableSnapshotStore } from '@server/compose/table-refresh'

const ISO_DATE_LENGTH = 10

type RemoteTableBody = Omit<SnapshotTable, 'attribution'>

type RememberedRemoteTable = { headerSha256: string; dataSha256: string; body: RemoteTableBody | null }

const parseRemoteTable = (definition: TableSourceDefinition, row: RemoteSnapshotRow): RemoteTableBody | null => {
    const headerJson = parseJsonBytes(row.headerBytes)
    const dataJson = parseJsonBytes(row.dataBytes)
    if (!headerJson.ok || !dataJson.ok) return null
    const header = remoteHeaderSchema.safeParse(headerJson.value)
    const entries = remoteEntriesSchema.safeParse(dataJson.value)
    if (!header.success || !entries.success) return null
    const text = new TextDecoder().decode(row.dataBytes)
    const fetchedAt = new Date(row.fetchedAt).toISOString()
    return {
        id: definition.id,
        header: {
            name: header.data.name,
            symbol: header.data.symbol,
            data_url: '',
            level_order: deriveLevelOrder(header.data, entries.data),
            last_update: fetchedAt.slice(0, ISO_DATE_LENGTH),
        },
        entries: entries.data.map(toSnapshotEntry),
        standardData: () => text,
        source: {
            origin: { page_url: definition.pageUrl, header_url: row.headerUrl },
            fetched_at: fetchedAt,
            snapshot: {
                header_file_sha256: row.headerSha256,
                data_file_sha256: row.dataSha256,
                entries: entries.data.length,
                levels: new Set(entries.data.map((entry) => entry.level)).size,
            },
        },
        course: header.data.course ?? null,
    }
}

const withAttribution = (definition: TableSourceDefinition, version: RemoteSnapshotVersion, body: RemoteTableBody): SnapshotTable => ({
    ...body,
    attribution: {
        sourceName: definition.sourceName,
        maker: definition.maker,
        pageUrl: definition.pageUrl,
        fetchedAt: version.fetchedAt,
        checkedAt: version.checkedAt,
        dataSha256: version.dataSha256,
    },
})

/**
 * The tables copied from other sites, read from the database. Each call reads only the small version rows; a copy is decompressed and
 * parsed again only when its hashes differ from the ones this instance remembers, so a serverless instance unpacks a copy once.
 */
export const createRemoteSnapshotCache = (store: Pick<TableSnapshotStoreDb, 'listVersions' | 'loadSnapshot'>) => {
    let remembered = new Map<string, RememberedRemoteTable>()

    const read = async (definition: TableSourceDefinition, version: RemoteSnapshotVersion): Promise<RememberedRemoteTable | null> => {
        const row = await store.loadSnapshot(version.tableId)
        if (!row) return null
        const body = parseRemoteTable(definition, row)
        if (!body) console.error('remote table copy is unreadable', { tableId: row.tableId })
        return { headerSha256: row.headerSha256, dataSha256: row.dataSha256, body }
    }

    return {
        load: async () => {
            const versions = new Map((await store.listVersions()).map((version) => [version.tableId, version]))
            const current = new Map<string, RememberedRemoteTable>()
            const tables: SnapshotTable[] = []
            for (const definition of REMOTE_TABLES) {
                const version = versions.get(definition.id)
                if (!version) continue
                const known = remembered.get(version.tableId)
                const isRemembered = known && known.headerSha256 === version.headerSha256 && known.dataSha256 === version.dataSha256
                const table = isRemembered ? known : await read(definition, version)
                if (!table) continue
                current.set(version.tableId, table)
                if (table.body) tables.push(withAttribution(definition, version, table.body))
            }
            remembered = current
            return tables
        },
    }
}

/** Every table the server serves: the bundled ones, then the copies of other sites. */
export const createTableLoader = (remoteTables: Pick<ReturnType<typeof createRemoteSnapshotCache>, 'load'>) => async () =>
    new Map([...getSnapshotTables(), ...(await remoteTables.load()).map((table) => [table.id, table] as const)])

export const composeTableSnapshot = (db: Database) => {
    const tableSnapshotService = createTableSnapshotService({
        loadTables: createTableLoader(createRemoteSnapshotCache(createTableSnapshotStore(db))),
        db: {
            listBestByMd5: async ({ userId, md5s }) => {
                if (md5s.length === 0) return []
                const rows = await db
                    .select({
                        md5: chart.md5,
                        clear: chartBest.clear,
                        assistGrade: chartBest.assistGrade,
                        exScore: chartBest.exScore,
                        minbp: chartBest.minbp,
                        maxCombo: chartBest.maxCombo,
                        totalNotes: score.notes,
                        playedAt: score.playedAt,
                        scratchAuto: chartBest.scratchAuto,
                    })
                    .from(chartBest)
                    .innerJoin(chart, eq(chart.sha256, chartBest.chartSha256))
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .where(and(eq(chartBest.userId, userId), chartBestScope(db, true), inArray(chart.md5, md5s)))
                return rows.flatMap(({ md5, ...record }) => (md5 ? [{ md5, ...record }] : []))
            },
        },
    })

    return { tableSnapshotService }
}
