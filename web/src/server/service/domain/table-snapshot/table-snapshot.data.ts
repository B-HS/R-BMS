import insane1Data from '../../../../../data/tables/insane1/data.json'
import insane1Header from '../../../../../data/tables/insane1/header.json'
import insane1Source from '../../../../../data/tables/insane1/source.json'
import { snapshotEntriesSchema, snapshotHeaderSchema, snapshotSourceSchema } from '@server/dto/table-snapshot'
import type { SnapshotEntry, SnapshotHeader, SnapshotSource } from '@server/dto/table-snapshot'

/** Where a table copied from another site came from and when this server last downloaded it. */
export type TableAttribution = {
    sourceName: string
    maker: string
    pageUrl: string
    fetchedAt: number
    checkedAt: number
    dataSha256: string
}

export type SnapshotTable = {
    id: string
    header: SnapshotHeader
    entries: SnapshotEntry[]
    /** The text of the standard `data.json`: the committed file for a bundled table, the downloaded bytes for a copied one. */
    standardData: () => string
    source: SnapshotSource
    /** Null for a bundled table, which carries no trace of the site it was taken from. */
    attribution: TableAttribution | null
    /** The `course` list of the header as its maker wrote it, or null when the table has none. */
    course: unknown
}

const RAW_SNAPSHOTS: { id: string; header: unknown; entries: unknown; source: unknown }[] = [
    { id: 'insane1', header: insane1Header, entries: insane1Data, source: insane1Source },
]

const serialiseOnce = (entries: unknown) => {
    let text: string | null = null
    return () => {
        text ??= JSON.stringify(entries)
        return text
    }
}

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
                standardData: serialiseOnce(raw.entries),
                source: snapshotSourceSchema.parse(raw.source),
                attribution: null,
                course: null,
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
