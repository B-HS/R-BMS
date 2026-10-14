import 'server-only'
import { getSnapshotTables } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { groupTableByLevel } from '@server/service/domain/table-snapshot/table-snapshot.service'

/** Builds the compact table copy the folder editor searches and imports from, read straight from the bundled snapshot. */
export const readFolderTableIndex = () =>
    [...getSnapshotTables().values()].map((table) => {
        const detail = groupTableByLevel(table)
        return {
            id: detail.table.id,
            name: detail.table.name,
            symbol: detail.table.symbol,
            levels: detail.levels.map((group) => ({
                level: group.level,
                entries: group.entries.map((entry) => ({ md5: entry.md5, title: entry.title, artist: entry.artist })),
            })),
        }
    })
