import type { FolderChartText, FolderItemRow, FolderRow, FolderServiceDb } from '@server/service/domain/folder/folder.service'

type StoredFolder = FolderRow & { userId: string }

export const createMemoryFolderDb = (charts: FolderChartText[] = []) => {
    const folders = new Map<string, StoredFolder>()
    const items = new Map<string, FolderItemRow[]>()

    const toRow = (folder: StoredFolder): FolderRow => ({
        id: folder.id,
        name: folder.name,
        description: folder.description,
        isPublic: folder.isPublic,
        position: folder.position,
        itemCount: folder.itemCount,
        updatedAt: folder.updatedAt,
    })

    const ownedFolders = (userId: string) =>
        [...folders.values()].filter((folder) => folder.userId === userId).sort((a, b) => a.position - b.position)

    const touch = (folderId: string, updatedAt: number) => {
        const folder = folders.get(folderId)
        if (folder) folders.set(folderId, { ...folder, itemCount: items.get(folderId)?.length ?? 0, updatedAt })
    }

    const db: FolderServiceDb = {
        listFolders: async (userId) => ownedFolders(userId).map(toRow),

        findFolder: async ({ userId, folderId }) => {
            const folder = folders.get(folderId)
            return folder && folder.userId === userId ? toRow(folder) : null
        },

        listItems: async (folderId) => [...(items.get(folderId) ?? [])].sort((a, b) => a.position - b.position),

        sumItemCount: async (userId) => ownedFolders(userId).reduce((sum, folder) => sum + folder.itemCount, 0),

        insertFolder: async (row) => {
            if (ownedFolders(row.userId).some((folder) => folder.name.toLowerCase() === row.name.toLowerCase())) return 'name_taken'
            folders.set(row.id, { ...row, isPublic: false, itemCount: 0 })
            items.set(row.id, [])
            return 'created'
        },

        updateFolder: async ({ userId, folderId, name, description, updatedAt }) => {
            const folder = folders.get(folderId)
            if (!folder || folder.userId !== userId) return 'missing'
            if (
                name !== undefined &&
                ownedFolders(userId).some((other) => other.id !== folderId && other.name.toLowerCase() === name.toLowerCase())
            ) {
                return 'name_taken'
            }
            folders.set(folderId, { ...folder, name: name ?? folder.name, description: description ?? folder.description, updatedAt })
            return 'updated'
        },

        deleteFolder: async ({ userId, folderId }) => {
            const folder = folders.get(folderId)
            if (!folder || folder.userId !== userId) return false
            folders.delete(folderId)
            items.delete(folderId)
            for (const [index, other] of ownedFolders(userId).entries()) folders.set(other.id, { ...other, position: index })
            return true
        },

        reorderFolders: async ({ userId, folderIds, updatedAt }) => {
            for (const [index, folderId] of folderIds.entries()) {
                const folder = folders.get(folderId)
                if (folder && folder.userId === userId && folder.position !== index) folders.set(folderId, { ...folder, position: index, updatedAt })
            }
        },

        appendItems: async ({ folderId, items: added, updatedAt }) => {
            const current = items.get(folderId) ?? []
            const start = current.length === 0 ? 0 : Math.max(...current.map((item) => item.position)) + 1
            items.set(folderId, [...current, ...added.map((item, index) => ({ ...item, position: start + index }))])
            touch(folderId, updatedAt)
            return { itemCount: items.get(folderId)?.length ?? 0 }
        },

        removeItems: async ({ folderId, md5s, updatedAt }) => {
            const current = items.get(folderId) ?? []
            const kept = current.filter((item) => !md5s.includes(item.md5))
            items.set(folderId, kept)
            if (kept.length !== current.length) touch(folderId, updatedAt)
            return { removed: current.length - kept.length, itemCount: kept.length }
        },

        reorderItems: async ({ folderId, md5s, updatedAt }) => {
            const current = items.get(folderId) ?? []
            items.set(
                folderId,
                md5s.flatMap((md5, index) => {
                    const found = current.find((item) => item.md5 === md5)
                    return found ? [{ ...found, position: index }] : []
                }),
            )
            touch(folderId, updatedAt)
        },

        replaceItems: async ({ folderId, items: replacement, updatedAt }) => {
            items.set(
                folderId,
                replacement.map((item, index) => ({ ...item, position: index })),
            )
            touch(folderId, updatedAt)
            return { itemCount: replacement.length }
        },

        updateItemNote: async ({ folderId, md5, note, updatedAt }) => {
            const current = items.get(folderId) ?? []
            if (!current.some((item) => item.md5 === md5)) return false
            items.set(
                folderId,
                current.map((item) => (item.md5 === md5 ? { ...item, note } : item)),
            )
            touch(folderId, updatedAt)
            return true
        },

        listFolderIdsContaining: async ({ userId, md5 }) =>
            ownedFolders(userId)
                .filter((folder) => (items.get(folder.id) ?? []).some((item) => item.md5 === md5))
                .map((folder) => folder.id),

        findChartsByMd5: async (md5s) => charts.filter((chart) => md5s.includes(chart.md5)),
    }

    return { db, folders, items }
}
