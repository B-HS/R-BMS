import insane1Data from '../../../../../data/tables/insane1/data.json'
import insane1Header from '../../../../../data/tables/insane1/header.json'
import insane1Source from '../../../../../data/tables/insane1/source.json'
import { snapshotEntriesSchema, snapshotHeaderSchema, snapshotSourceSchema } from '@server/dto/table-snapshot'
import type { SnapshotEntry, SnapshotHeader, SnapshotSource } from '@server/dto/table-snapshot'

export type SnapshotTable = {
    id: string
    header: SnapshotHeader
    entries: SnapshotEntry[]
    rawEntries: unknown
    source: SnapshotSource
}

const RAW_SNAPSHOTS: { id: string; header: unknown; entries: unknown; source: unknown }[] = [
    { id: 'insane1', header: insane1Header, entries: insane1Data, source: insane1Source },
]

let loaded: Map<string, SnapshotTable> | null = null

export const getSnapshotTables = () => {
    if (loaded) return loaded
    loaded = new Map(
        RAW_SNAPSHOTS.map((raw) => [
            raw.id,
            {
                id: raw.id,
                header: snapshotHeaderSchema.parse(raw.header),
                entries: snapshotEntriesSchema.parse(raw.entries),
                rawEntries: raw.entries,
                source: snapshotSourceSchema.parse(raw.source),
            },
        ]),
    )
    return loaded
}

export const getSnapshotTable = (tableId: string) => getSnapshotTables().get(tableId) ?? null

let md5Index: Map<string, { title: string; artist: string }> | null = null

export const findSnapshotTextByMd5 = (md5: string) => {
    if (!md5Index) {
        md5Index = new Map()
        for (const table of getSnapshotTables().values()) {
            for (const entry of table.entries) {
                if (!md5Index.has(entry.md5)) md5Index.set(entry.md5, { title: entry.title, artist: entry.artist })
            }
        }
    }
    return md5Index.get(md5) ?? null
}
