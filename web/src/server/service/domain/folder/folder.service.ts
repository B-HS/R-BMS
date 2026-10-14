import {
    FOLDER_ID_PREFIX,
    FOLDER_LIMIT,
    type FolderCreateInput,
    type FolderDetailOutput,
    type FolderItemChangeOutput,
    type FolderItemInput,
    type FolderItemOutput,
    type FolderItemsReplaceInput,
    type FolderSummaryOutput,
    type FolderUpdateInput,
} from '@server/dto/folder'

export type FolderRow = {
    id: string
    name: string
    description: string
    isPublic: boolean
    position: number
    itemCount: number
    updatedAt: number
}

export type FolderItemRow = {
    md5: string
    sha256: string | null
    title: string
    artist: string
    note: string
    position: number
}

export type FolderChartText = {
    md5: string
    sha256: string
    title: string
    artist: string
}

export type TableImportEntry = {
    md5: string
    sha256: string
    title: string
    artist: string
    level: string
}

export type FolderServiceDb = {
    listFolders: (userId: string) => Promise<FolderRow[]>
    findFolder: (params: { userId: string; folderId: string }) => Promise<FolderRow | null>
    listItems: (folderId: string) => Promise<FolderItemRow[]>
    sumItemCount: (userId: string) => Promise<number>
    insertFolder: (row: {
        id: string
        userId: string
        name: string
        description: string
        position: number
        updatedAt: number
    }) => Promise<'created' | 'name_taken'>
    updateFolder: (params: {
        userId: string
        folderId: string
        name?: string
        description?: string
        updatedAt: number
    }) => Promise<'updated' | 'name_taken' | 'missing'>
    deleteFolder: (params: { userId: string; folderId: string }) => Promise<boolean>
    reorderFolders: (params: { userId: string; folderIds: string[]; updatedAt: number }) => Promise<void>
    /**
     * Adds the items a folder does not hold yet as one unit with the folder row locked, so two adds at once neither collide on a
     * song nor share a position. `added` counts the rows really written; a result that would pass `maxItems` writes nothing.
     */
    appendItems: (params: {
        folderId: string
        items: Omit<FolderItemRow, 'position'>[]
        updatedAt: number
        maxItems: number
    }) => Promise<{ status: 'added'; added: number; itemCount: number; updatedAt: number } | { status: 'folder_full' } | { status: 'missing' }>
    removeItems: (params: { folderId: string; md5s: string[]; updatedAt: number }) => Promise<{ removed: number; itemCount: number }>
    reorderItems: (params: { folderId: string; md5s: string[]; updatedAt: number }) => Promise<void>
    /**
     * Replaces the songs, and the name and description when given, as one unit with the folder row locked. A stale
     * `baseUpdatedAt` is judged against the locked row, so a save made on an older copy never overwrites a newer one.
     */
    replaceItems: (params: {
        userId: string
        folderId: string
        items: Omit<FolderItemRow, 'position'>[]
        updatedAt: number
        baseUpdatedAt?: number
        meta: { name?: string; description?: string }
    }) => Promise<
        | { status: 'replaced'; itemCount: number; updatedAt: number }
        | { status: 'conflict'; serverUpdatedAt: number }
        | { status: 'name_taken' }
        | { status: 'missing' }
    >
    updateItemNote: (params: { folderId: string; md5: string; note: string; updatedAt: number }) => Promise<boolean>
    listFolderIdsContaining: (params: { userId: string; md5: string }) => Promise<string[]>
    findChartsByMd5: (md5s: string[]) => Promise<FolderChartText[]>
}

export type FolderServiceDeps = {
    db: FolderServiceDb
    newId: (prefix: string) => string
    now: () => number
    loadTableEntries: (tableId: string) => TableImportEntry[] | null
    lookupTableText: (md5: string) => { title: string; artist: string } | null
}

export type FolderFailure =
    | { reason: 'not_found' }
    | { reason: 'name_taken' }
    | {
          reason: 'limit_exceeded'
          details: { scope: 'folders' | 'items_per_folder' | 'items_per_user'; limit: number; current: number; requested: number }
      }
    | { reason: 'set_mismatch' }
    | { reason: 'conflict'; details: { server_updated_at: number } }
    | { reason: 'table_not_found' }

export type FolderResult<T> = { ok: true; value: T } | ({ ok: false } & FolderFailure)

const succeed = <T>(value: T): FolderResult<T> => ({ ok: true, value })

const fail = <T>(failure: FolderFailure): FolderResult<T> => ({ ok: false, ...failure })

const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f]/g

export const sanitiseCopiedText = (value: string, max = FOLDER_LIMIT.TEXT_COPY_MAX) =>
    [...value.replace(CONTROL_CHARACTERS, '').trim()].slice(0, max).join('')

export const nextStamp = (now: number, previous: number) => Math.max(now, previous + 1)

export const toFolderSummary = (row: FolderRow): FolderSummaryOutput => ({
    id: row.id,
    name: row.name,
    description: row.description,
    is_public: row.isPublic,
    position: row.position,
    item_count: row.itemCount,
    updated_at: row.updatedAt,
})

export const toFolderItem = (row: Pick<FolderItemRow, 'md5' | 'sha256' | 'title' | 'artist' | 'note'>): FolderItemOutput => ({
    md5: row.md5,
    sha256: row.sha256 ?? '',
    title: row.title,
    artist: row.artist,
    note: row.note,
})

export const toFolderDetail = (row: FolderRow, items: FolderItemRow[]): FolderDetailOutput => ({
    ...toFolderSummary(row),
    items: items.map(toFolderItem),
})

export const uniqueByMd5 = <T extends { md5: string }>(items: T[]) => {
    const seen = new Set<string>()
    return items.filter((item) => {
        if (seen.has(item.md5)) return false
        seen.add(item.md5)
        return true
    })
}

export const isSameSet = (current: string[], requested: string[]) => {
    if (current.length !== requested.length) return false
    const known = new Set(current)
    return new Set(requested).size === requested.length && requested.every((value) => known.has(value))
}

export const planItemInsert = (params: { existingMd5s: string[]; incoming: FolderItemInput[]; folderCount: number; userCount: number }) => {
    const existing = new Set(params.existingMd5s)
    const fresh = uniqueByMd5(params.incoming).filter((item) => !existing.has(item.md5))
    const skipped = params.incoming.length - fresh.length
    const folderRoom = FOLDER_LIMIT.ITEMS_PER_FOLDER - params.folderCount
    const userRoom = FOLDER_LIMIT.ITEMS_PER_USER - params.userCount
    if (fresh.length > folderRoom) {
        return {
            fresh,
            skipped,
            exceeded: {
                scope: 'items_per_folder' as const,
                limit: FOLDER_LIMIT.ITEMS_PER_FOLDER,
                current: params.folderCount,
                requested: fresh.length,
            },
        }
    }
    if (fresh.length > userRoom) {
        return {
            fresh,
            skipped,
            exceeded: { scope: 'items_per_user' as const, limit: FOLDER_LIMIT.ITEMS_PER_USER, current: params.userCount, requested: fresh.length },
        }
    }
    return { fresh, skipped, exceeded: null }
}

const firstFilled = (...candidates: (string | undefined)[]) => candidates.find((value) => value !== undefined && value !== '') ?? ''

export const resolveItemText = (params: {
    input: Pick<FolderItemInput, 'sha256' | 'title' | 'artist'>
    chart: FolderChartText | undefined
    tableText: { title: string; artist: string } | null
}) => ({
    sha256: params.chart?.sha256 || params.input.sha256 || null,
    title: sanitiseCopiedText(firstFilled(params.chart?.title, params.tableText?.title, params.input.title)),
    artist: sanitiseCopiedText(firstFilled(params.chart?.artist, params.tableText?.artist, params.input.artist)),
})

export const createFolderService = (deps: FolderServiceDeps) => {
    const buildNewItems = async (incoming: FolderItemInput[]) => {
        const charts = new Map((await deps.db.findChartsByMd5(incoming.map((item) => item.md5))).map((chart) => [chart.md5, chart]))
        return incoming.map((input) => ({
            md5: input.md5,
            ...resolveItemText({ input, chart: charts.get(input.md5), tableText: deps.lookupTableText(input.md5) }),
            note: input.note,
        }))
    }

    const withFolder = async <T>(params: { userId: string; folderId: string }, run: (folder: FolderRow) => Promise<FolderResult<T>>) => {
        const folder = await deps.db.findFolder(params)
        if (!folder) return fail<T>({ reason: 'not_found' })
        return run(folder)
    }

    const insertFresh = async (params: { folder: FolderRow; userId: string; incoming: FolderItemInput[] }) => {
        const [existing, userCount] = await Promise.all([deps.db.listItems(params.folder.id), deps.db.sumItemCount(params.userId)])
        const plan = planItemInsert({
            existingMd5s: existing.map((item) => item.md5),
            incoming: params.incoming,
            folderCount: params.folder.itemCount,
            userCount,
        })
        if (plan.exceeded) return fail<FolderItemChangeOutput>({ reason: 'limit_exceeded', details: plan.exceeded })
        if (plan.fresh.length === 0) {
            return succeed({
                added: 0,
                removed: 0,
                skipped: plan.skipped,
                item_count: params.folder.itemCount,
                updated_at: params.folder.updatedAt,
            })
        }
        const outcome = await deps.db.appendItems({
            folderId: params.folder.id,
            items: await buildNewItems(plan.fresh),
            updatedAt: nextStamp(deps.now(), params.folder.updatedAt),
            maxItems: FOLDER_LIMIT.ITEMS_PER_FOLDER,
        })
        if (outcome.status === 'missing') return fail<FolderItemChangeOutput>({ reason: 'not_found' })
        if (outcome.status === 'folder_full') {
            return fail<FolderItemChangeOutput>({
                reason: 'limit_exceeded',
                details: {
                    scope: 'items_per_folder',
                    limit: FOLDER_LIMIT.ITEMS_PER_FOLDER,
                    current: params.folder.itemCount,
                    requested: plan.fresh.length,
                },
            })
        }
        return succeed({
            added: outcome.added,
            removed: 0,
            skipped: params.incoming.length - outcome.added,
            item_count: outcome.itemCount,
            updated_at: outcome.updatedAt,
        })
    }

    const list = async (userId: string) => (await deps.db.listFolders(userId)).map(toFolderSummary)

    return {
        list,

        get: async (params: { userId: string; folderId: string }) => {
            const folder = await deps.db.findFolder(params)
            if (!folder) return null
            return toFolderDetail(folder, await deps.db.listItems(folder.id))
        },

        create: async (params: { userId: string; input: FolderCreateInput }): Promise<FolderResult<FolderSummaryOutput>> => {
            const folders = await deps.db.listFolders(params.userId)
            if (folders.length >= FOLDER_LIMIT.FOLDERS_PER_USER) {
                return fail({
                    reason: 'limit_exceeded',
                    details: { scope: 'folders', limit: FOLDER_LIMIT.FOLDERS_PER_USER, current: folders.length, requested: 1 },
                })
            }
            const row = {
                id: deps.newId(FOLDER_ID_PREFIX),
                userId: params.userId,
                name: params.input.name,
                description: params.input.description,
                position: folders.length,
                updatedAt: deps.now(),
            }
            if ((await deps.db.insertFolder(row)) === 'name_taken') return fail({ reason: 'name_taken' })
            return succeed(toFolderSummary({ ...row, isPublic: false, itemCount: 0 }))
        },

        update: async (params: { userId: string; folderId: string; input: FolderUpdateInput }) =>
            withFolder<FolderSummaryOutput>(params, async (folder) => {
                const updatedAt = nextStamp(deps.now(), folder.updatedAt)
                const outcome = await deps.db.updateFolder({
                    userId: params.userId,
                    folderId: folder.id,
                    name: params.input.name,
                    description: params.input.description,
                    updatedAt,
                })
                if (outcome === 'name_taken') return fail({ reason: 'name_taken' })
                if (outcome === 'missing') return fail({ reason: 'not_found' })
                return succeed(
                    toFolderSummary({
                        ...folder,
                        name: params.input.name ?? folder.name,
                        description: params.input.description ?? folder.description,
                        updatedAt,
                    }),
                )
            }),

        remove: async (params: { userId: string; folderId: string }) => deps.db.deleteFolder(params),

        reorderFolders: async (params: { userId: string; folderIds: string[] }): Promise<FolderResult<FolderSummaryOutput[]>> => {
            const folders = await deps.db.listFolders(params.userId)
            const currentIds = folders.map((folder) => folder.id)
            if (!isSameSet(currentIds, params.folderIds)) return fail({ reason: 'set_mismatch' })
            await deps.db.reorderFolders({ userId: params.userId, folderIds: params.folderIds, updatedAt: deps.now() })
            return succeed(await list(params.userId))
        },

        addItems: async (params: { userId: string; folderId: string; items: FolderItemInput[] }) =>
            withFolder<FolderItemChangeOutput>(params, (folder) => insertFresh({ folder, userId: params.userId, incoming: params.items })),

        removeItems: async (params: { userId: string; folderId: string; md5s: string[] }) =>
            withFolder<FolderItemChangeOutput>(params, async (folder) => {
                const updatedAt = nextStamp(deps.now(), folder.updatedAt)
                const { removed, itemCount } = await deps.db.removeItems({ folderId: folder.id, md5s: [...new Set(params.md5s)], updatedAt })
                return succeed({
                    added: 0,
                    removed,
                    skipped: 0,
                    item_count: itemCount,
                    updated_at: removed > 0 ? updatedAt : folder.updatedAt,
                })
            }),

        reorderItems: async (params: { userId: string; folderId: string; md5s: string[] }) =>
            withFolder<FolderItemChangeOutput>(params, async (folder) => {
                const current = await deps.db.listItems(folder.id)
                if (
                    !isSameSet(
                        current.map((item) => item.md5),
                        params.md5s,
                    )
                )
                    return fail({ reason: 'set_mismatch' })
                const updatedAt = nextStamp(deps.now(), folder.updatedAt)
                await deps.db.reorderItems({ folderId: folder.id, md5s: params.md5s, updatedAt })
                return succeed({ added: 0, removed: 0, skipped: 0, item_count: folder.itemCount, updated_at: updatedAt })
            }),

        replaceItems: async (params: { userId: string; folderId: string; input: FolderItemsReplaceInput }) =>
            withFolder<FolderItemChangeOutput>(params, async (folder) => {
                if (params.input.base_updated_at !== undefined && params.input.base_updated_at !== folder.updatedAt) {
                    return fail({ reason: 'conflict', details: { server_updated_at: folder.updatedAt } })
                }
                const [existing, userCount] = await Promise.all([deps.db.listItems(folder.id), deps.db.sumItemCount(params.userId)])
                const requested = uniqueByMd5(params.input.items)
                const known = new Map(existing.map((item) => [item.md5, item]))
                const fresh = requested.filter((item) => !known.has(item.md5))
                const userAfter = userCount - folder.itemCount + requested.length
                if (userAfter > FOLDER_LIMIT.ITEMS_PER_USER) {
                    return fail({
                        reason: 'limit_exceeded',
                        details: { scope: 'items_per_user', limit: FOLDER_LIMIT.ITEMS_PER_USER, current: userCount, requested: requested.length },
                    })
                }
                const created = new Map((await buildNewItems(fresh)).map((item) => [item.md5, item]))
                const items = requested.flatMap((input) => {
                    const stored = known.get(input.md5)
                    if (stored) return [{ md5: stored.md5, sha256: stored.sha256, title: stored.title, artist: stored.artist, note: input.note }]
                    const made = created.get(input.md5)
                    return made ? [made] : []
                })
                const outcome = await deps.db.replaceItems({
                    userId: params.userId,
                    folderId: folder.id,
                    items,
                    updatedAt: nextStamp(deps.now(), folder.updatedAt),
                    baseUpdatedAt: params.input.base_updated_at,
                    meta: { name: params.input.name, description: params.input.description },
                })
                if (outcome.status === 'missing') return fail({ reason: 'not_found' })
                if (outcome.status === 'name_taken') return fail({ reason: 'name_taken' })
                if (outcome.status === 'conflict') return fail({ reason: 'conflict', details: { server_updated_at: outcome.serverUpdatedAt } })
                const keptMd5s = new Set(requested.map((item) => item.md5))
                return succeed({
                    added: fresh.length,
                    removed: existing.filter((item) => !keptMd5s.has(item.md5)).length,
                    skipped: params.input.items.length - requested.length,
                    item_count: outcome.itemCount,
                    updated_at: outcome.updatedAt,
                })
            }),

        updateItemNote: async (params: { userId: string; folderId: string; md5: string; note: string }) =>
            withFolder<FolderItemOutput>(params, async (folder) => {
                const items = await deps.db.listItems(folder.id)
                const item = items.find((candidate) => candidate.md5 === params.md5)
                if (!item) return fail({ reason: 'not_found' })
                const updatedAt = nextStamp(deps.now(), folder.updatedAt)
                await deps.db.updateItemNote({ folderId: folder.id, md5: params.md5, note: params.note, updatedAt })
                return succeed(toFolderItem({ ...item, note: params.note }))
            }),

        importFromTable: async (params: { userId: string; folderId: string; tableId: string; levels?: string[] }) =>
            withFolder<FolderItemChangeOutput>(params, async (folder) => {
                const entries = deps.loadTableEntries(params.tableId)
                if (!entries) return fail({ reason: 'table_not_found' })
                const wanted = params.levels && params.levels.length > 0 ? new Set(params.levels) : null
                const incoming = entries
                    .filter((entry) => wanted === null || wanted.has(entry.level))
                    .map((entry) => ({ md5: entry.md5, sha256: entry.sha256, title: entry.title, artist: entry.artist, note: '' }))
                return insertFresh({ folder, userId: params.userId, incoming })
            }),

        listContaining: async (params: { userId: string; md5: string }) => deps.db.listFolderIdsContaining(params),
    }
}

export type FolderService = ReturnType<typeof createFolderService>
