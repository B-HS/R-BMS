import { FOLDER_LIMIT, toNewFolderItemInput, type FolderDetail, type FolderItem, type FolderTableIndex } from '@entities/folder.type'
import type { FolderChartMove } from '@features/folder/folder-chart-row'

type FolderDraft = Pick<FolderDetail, 'name' | 'description' | 'items'>

export type FolderBase = FolderDraft & Pick<FolderDetail, 'updated_at'>

export const toFolderBase = (folder: FolderDetail) => ({
    name: folder.name,
    description: folder.description,
    items: folder.items,
    updated_at: folder.updated_at,
})

export const toFolderDraft = (folder: FolderDraft) => ({ name: folder.name, description: folder.description, items: folder.items })

const commonOrder = (items: FolderItem[], keep: ReadonlySet<string>) => items.filter((item) => keep.has(item.md5)).map((item) => item.md5)

const isSameSequence = (left: string[], right: string[]) => left.length === right.length && left.every((value, index) => value === right[index])

/** Counts what an explicit save would change, so the header and the leave warning can say how much is unsaved. */
export const countFolderDraftChanges = (base: FolderDraft, draft: FolderDraft) => {
    const baseByMd5 = new Map(base.items.map((item) => [item.md5, item]))
    const draftMd5s = new Set(draft.items.map((item) => item.md5))
    const kept = new Set(draft.items.filter((item) => baseByMd5.has(item.md5)).map((item) => item.md5))
    const added = draft.items.length - kept.size
    const removed = base.items.filter((item) => !draftMd5s.has(item.md5)).length
    const notes = draft.items.filter((item) => {
        const stored = baseByMd5.get(item.md5)
        return stored !== undefined && stored.note !== item.note
    }).length
    const isReordered = !isSameSequence(commonOrder(base.items, kept), commonOrder(draft.items, kept))
    const isRenamed = base.name !== draft.name
    const isDescriptionChanged = base.description !== draft.description
    const total = added + removed + notes + Number(isReordered) + Number(isRenamed) + Number(isDescriptionChanged)
    return { added, removed, notes, isReordered, isRenamed, isDescriptionChanged, total }
}

export const describeFolderDraftChanges = (changes: ReturnType<typeof countFolderDraftChanges>) =>
    [
        changes.added > 0 && `추가 ${changes.added}`,
        changes.removed > 0 && `빼기 ${changes.removed}`,
        changes.isReordered && '순서',
        changes.notes > 0 && `메모 ${changes.notes}`,
        changes.isRenamed && '이름',
        changes.isDescriptionChanged && '설명',
    ]
        .filter((value) => typeof value === 'string')
        .join(' · ')

/** Moves one item and returns a new array; out-of-range moves leave the list unchanged. */
export const moveFolderItem = (items: FolderItem[], index: number, move: FolderChartMove) => {
    const targetByMove = { up: index - 1, down: index + 1, top: 0, bottom: items.length - 1 }
    const target = targetByMove[move]
    if (index < 0 || index >= items.length || target < 0 || target >= items.length || target === index) return items
    const moving = items[index]
    const rest = items.filter((_, position) => position !== index)
    return [...rest.slice(0, target), moving, ...rest.slice(target)]
}

/** Appends charts that are not in the list yet, stopping at the per-folder limit. */
export const appendFolderItems = (items: FolderItem[], incoming: (Pick<FolderItem, 'md5' | 'title' | 'artist'> & Partial<FolderItem>)[]) => {
    const present = new Set(items.map((item) => item.md5))
    const room = Math.max(0, FOLDER_LIMIT.ITEMS_PER_FOLDER - items.length)
    const fresh = [...new Map(incoming.filter((item) => !present.has(item.md5)).map((item) => [item.md5, item])).values()].slice(0, room)
    const appended = fresh.map((item) => ({
        md5: item.md5,
        sha256: item.sha256 ?? '',
        title: item.title,
        artist: item.artist,
        note: item.note ?? '',
    }))
    return { items: [...items, ...appended], added: appended.length, skipped: incoming.length - appended.length }
}

/** Builds the PUT body: stored charts send only md5 and note, new charts also carry their text so the server can keep it. */
export const toReplaceItems = (base: FolderDraft, draft: FolderDraft) => {
    const stored = new Set(base.items.map((item) => item.md5))
    return draft.items.map((item) => (stored.has(item.md5) ? { md5: item.md5, note: item.note } : toNewFolderItemInput(item)))
}

export const summariseFolderConflict = (draft: FolderDraft, server: FolderDetail) => {
    const serverMd5s = new Set(server.items.map((item) => item.md5))
    const draftMd5s = new Set(draft.items.map((item) => item.md5))
    const kept = new Set([...draftMd5s].filter((md5) => serverMd5s.has(md5)))
    return {
        serverName: server.name,
        serverItemCount: server.items.length,
        serverUpdatedAt: server.updated_at,
        onlyMine: draft.items.filter((item) => !serverMd5s.has(item.md5)).length,
        onlyServer: server.items.filter((item) => !draftMd5s.has(item.md5)).length,
        isOrderDifferent: !isSameSequence(commonOrder(server.items, kept), commonOrder(draft.items, kept)),
        isNameDifferent: server.name !== draft.name,
        isDescriptionDifferent: server.description !== draft.description,
    }
}

/** Maps every md5 in the bundled tables to its level label (for example `★12`), first table wins. */
export const buildLevelLabelByMd5 = (tables: FolderTableIndex[]) =>
    new Map(
        tables
            .flatMap((table) => table.levels.flatMap((group) => group.entries.map((entry) => [entry.md5, `${table.symbol}${group.level}`] as const)))
            .toReversed(),
    )
