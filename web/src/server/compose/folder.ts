import { and, asc, count, eq, inArray, max, sql } from 'drizzle-orm'
import type { Database } from '@server/db'
import { chart, userFolder, userFolderItem } from '@server/db/schema'
import { newId } from '@server/lib/token'
import { createFolderService } from '@server/service/domain/folder/folder.service'
import type { FolderItemRow, FolderRow } from '@server/service/domain/folder/folder.service'
import type { TableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'

const DUPLICATE_ENTRY_CODE = 'ER_DUP_ENTRY'

const hasDuplicateEntryCode = (error: unknown): boolean => {
    if (typeof error !== 'object' || error === null) return false
    const code = 'code' in error ? error.code : undefined
    if (code === DUPLICATE_ENTRY_CODE) return true
    return 'cause' in error ? hasDuplicateEntryCode(error.cause) : false
}

const folderSelection = {
    id: userFolder.id,
    name: userFolder.name,
    description: userFolder.description,
    isPublic: userFolder.isPublic,
    position: userFolder.position,
    itemCount: userFolder.itemCount,
    updatedAt: userFolder.updatedAt,
}

type ItemInsert = Omit<FolderItemRow, 'position'>

export const composeFolder = (
    db: Database,
    deps: { tableSnapshotService: TableSnapshotService; lookupTableText: (md5: string) => { title: string; artist: string } | null },
) => {
    const countItems = async (tx: Pick<Database, 'select'>, folderId: string) => {
        const [row] = await tx.select({ value: count() }).from(userFolderItem).where(eq(userFolderItem.folderId, folderId))
        return row?.value ?? 0
    }

    const toInsertRows = (folderId: string, items: ItemInsert[], startPosition: number) =>
        items.map((item, index) => ({
            folderId,
            md5: item.md5,
            sha256: item.sha256,
            title: item.title,
            artist: item.artist,
            note: item.note,
            position: startPosition + index,
        }))

    const folderService = createFolderService({
        newId,
        now: () => Date.now(),
        loadTableEntries: (tableId) => deps.tableSnapshotService.loadEntriesForImport(tableId),
        lookupTableText: deps.lookupTableText,
        db: {
            listFolders: async (userId): Promise<FolderRow[]> =>
                db.select(folderSelection).from(userFolder).where(eq(userFolder.userId, userId)).orderBy(asc(userFolder.position)),

            findFolder: async ({ userId, folderId }) => {
                const [row] = await db
                    .select(folderSelection)
                    .from(userFolder)
                    .where(and(eq(userFolder.id, folderId), eq(userFolder.userId, userId)))
                    .limit(1)
                return row ?? null
            },

            listItems: async (folderId) =>
                db
                    .select({
                        md5: userFolderItem.md5,
                        sha256: userFolderItem.sha256,
                        title: userFolderItem.title,
                        artist: userFolderItem.artist,
                        note: userFolderItem.note,
                        position: userFolderItem.position,
                    })
                    .from(userFolderItem)
                    .where(eq(userFolderItem.folderId, folderId))
                    .orderBy(asc(userFolderItem.position), asc(userFolderItem.md5)),

            sumItemCount: async (userId) => {
                const [row] = await db
                    .select({ value: sql<string | null>`sum(${userFolder.itemCount})` })
                    .from(userFolder)
                    .where(eq(userFolder.userId, userId))
                return Number(row?.value ?? 0)
            },

            insertFolder: async (row) => {
                try {
                    await db.insert(userFolder).values(row)
                    return 'created'
                } catch (error) {
                    if (hasDuplicateEntryCode(error)) return 'name_taken'
                    throw error
                }
            },

            updateFolder: async ({ userId, folderId, name, description, updatedAt }) => {
                try {
                    const [result] = await db
                        .update(userFolder)
                        .set({
                            ...(name === undefined ? {} : { name }),
                            ...(description === undefined ? {} : { description }),
                            updatedAt,
                        })
                        .where(and(eq(userFolder.id, folderId), eq(userFolder.userId, userId)))
                    return result.affectedRows > 0 ? 'updated' : 'missing'
                } catch (error) {
                    if (hasDuplicateEntryCode(error)) return 'name_taken'
                    throw error
                }
            },

            deleteFolder: async ({ userId, folderId }) =>
                db.transaction(async (tx) => {
                    const [result] = await tx.delete(userFolder).where(and(eq(userFolder.id, folderId), eq(userFolder.userId, userId)))
                    if (result.affectedRows === 0) return false
                    const remaining = await tx
                        .select({ id: userFolder.id, position: userFolder.position })
                        .from(userFolder)
                        .where(eq(userFolder.userId, userId))
                        .orderBy(asc(userFolder.position))
                    for (const [index, folder] of remaining.entries()) {
                        if (folder.position !== index) await tx.update(userFolder).set({ position: index }).where(eq(userFolder.id, folder.id))
                    }
                    return true
                }),

            reorderFolders: async ({ userId, folderIds, updatedAt }) =>
                db.transaction(async (tx) => {
                    const current = await tx
                        .select({ id: userFolder.id, position: userFolder.position })
                        .from(userFolder)
                        .where(eq(userFolder.userId, userId))
                    const positions = new Map(current.map((folder) => [folder.id, folder.position]))
                    for (const [index, folderId] of folderIds.entries()) {
                        if (positions.get(folderId) === index) continue
                        await tx
                            .update(userFolder)
                            .set({ position: index, updatedAt })
                            .where(and(eq(userFolder.id, folderId), eq(userFolder.userId, userId)))
                    }
                }),

            appendItems: async ({ folderId, items, updatedAt }) =>
                db.transaction(async (tx) => {
                    const [last] = await tx
                        .select({ value: max(userFolderItem.position) })
                        .from(userFolderItem)
                        .where(eq(userFolderItem.folderId, folderId))
                    await tx.insert(userFolderItem).values(toInsertRows(folderId, items, (last?.value ?? -1) + 1))
                    const itemCount = await countItems(tx, folderId)
                    await tx.update(userFolder).set({ itemCount, updatedAt }).where(eq(userFolder.id, folderId))
                    return { itemCount }
                }),

            removeItems: async ({ folderId, md5s, updatedAt }) =>
                db.transaction(async (tx) => {
                    const [result] = await tx
                        .delete(userFolderItem)
                        .where(and(eq(userFolderItem.folderId, folderId), inArray(userFolderItem.md5, md5s)))
                    const itemCount = await countItems(tx, folderId)
                    if (result.affectedRows > 0) await tx.update(userFolder).set({ itemCount, updatedAt }).where(eq(userFolder.id, folderId))
                    return { removed: result.affectedRows, itemCount }
                }),

            reorderItems: async ({ folderId, md5s, updatedAt }) =>
                db.transaction(async (tx) => {
                    if (md5s.length > 0) {
                        const cases = sql.join(
                            md5s.map((md5, index) => sql`when ${md5} then ${index}`),
                            sql` `,
                        )
                        await tx
                            .update(userFolderItem)
                            .set({ position: sql`case ${userFolderItem.md5} ${cases} end` })
                            .where(and(eq(userFolderItem.folderId, folderId), inArray(userFolderItem.md5, md5s)))
                    }
                    await tx.update(userFolder).set({ updatedAt }).where(eq(userFolder.id, folderId))
                }),

            replaceItems: async ({ folderId, items, updatedAt }) =>
                db.transaction(async (tx) => {
                    await tx.delete(userFolderItem).where(eq(userFolderItem.folderId, folderId))
                    if (items.length > 0) await tx.insert(userFolderItem).values(toInsertRows(folderId, items, 0))
                    await tx.update(userFolder).set({ itemCount: items.length, updatedAt }).where(eq(userFolder.id, folderId))
                    return { itemCount: items.length }
                }),

            updateItemNote: async ({ folderId, md5, note, updatedAt }) =>
                db.transaction(async (tx) => {
                    const [result] = await tx
                        .update(userFolderItem)
                        .set({ note })
                        .where(and(eq(userFolderItem.folderId, folderId), eq(userFolderItem.md5, md5)))
                    if (result.affectedRows === 0) return false
                    await tx.update(userFolder).set({ updatedAt }).where(eq(userFolder.id, folderId))
                    return true
                }),

            listFolderIdsContaining: async ({ userId, md5 }) => {
                const rows = await db
                    .select({ id: userFolder.id })
                    .from(userFolderItem)
                    .innerJoin(userFolder, eq(userFolder.id, userFolderItem.folderId))
                    .where(and(eq(userFolder.userId, userId), eq(userFolderItem.md5, md5)))
                    .orderBy(asc(userFolder.position))
                return rows.map((row) => row.id)
            },

            findChartsByMd5: async (md5s) => {
                if (md5s.length === 0) return []
                const rows = await db
                    .select({ md5: chart.md5, sha256: chart.sha256, title: chart.title, artist: chart.artist })
                    .from(chart)
                    .where(inArray(chart.md5, md5s))
                return rows.flatMap((row) => (row.md5 ? [{ md5: row.md5, sha256: row.sha256, title: row.title, artist: row.artist }] : []))
            },
        },
    })

    return { folderService }
}
