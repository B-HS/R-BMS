import type { ClientBuildInput } from '@server/dto/admin'

export type ClientBuildRow = {
    sha256: string
    version: string
    platform: string
    channel: string
    releasedAt: number
    trusted: boolean
    note: string
}

export type BuildServiceDb = {
    listBuilds: () => Promise<ClientBuildRow[]>
    upsertBuild: (row: ClientBuildRow) => Promise<void>
}

export const toClientBuild = (row: ClientBuildRow) => ({
    sha256: row.sha256,
    version: row.version,
    platform: row.platform,
    channel: row.channel,
    released_at: row.releasedAt,
    trusted: row.trusted,
    note: row.note,
})

export const createBuildService = (deps: { db: BuildServiceDb }) => ({
    list: async () => (await deps.db.listBuilds()).map(toClientBuild),

    upsert: async (input: ClientBuildInput) => {
        const row: ClientBuildRow = {
            sha256: input.sha256,
            version: input.version,
            platform: input.platform,
            channel: input.channel,
            releasedAt: input.released_at,
            trusted: input.trusted,
            note: input.note,
        }
        await deps.db.upsertBuild(row)
        return toClientBuild(row)
    },
})

export type BuildService = ReturnType<typeof createBuildService>
