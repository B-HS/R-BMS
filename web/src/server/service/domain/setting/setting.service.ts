import type { SettingPutInput } from '@server/dto/setting'

export type SettingRow = {
    name: string
    format: string
    content: string
    size: number
    updatedAt: number
}

export type SettingKeyRow = Omit<SettingRow, 'content'>

export type SettingServiceDb = {
    findSetting: (params: { userId: string; name: string }) => Promise<SettingRow | null>
    upsertSetting: (row: { userId: string; name: string; format: string; content: string; size: number; updatedAt: number }) => Promise<void>
    listSettings: (userId: string) => Promise<SettingKeyRow[]>
}

export type SettingServiceDeps = {
    db: SettingServiceDb
    now: () => number
}

export const toSettingBlob = (row: SettingRow) => ({
    name: row.name,
    format: row.format,
    content: row.content,
    updated_at: row.updatedAt,
})

export const toSettingKey = (row: SettingKeyRow) => ({
    key: row.name,
    format: row.format,
    updated_at: row.updatedAt,
    size: row.size,
})

export const isSettingConflict = (base: number | null, current: SettingRow | null) => base !== null && (current?.updatedAt ?? 0) !== base

export const createSettingService = (deps: SettingServiceDeps) => ({
    get: async (params: { userId: string; name: string }) => {
        const row = await deps.db.findSetting(params)
        return row ? toSettingBlob(row) : null
    },

    put: async (params: { userId: string; name: string; input: SettingPutInput }) => {
        const { input } = params
        const current = await deps.db.findSetting({ userId: params.userId, name: params.name })
        if (isSettingConflict(input.base_updated_at ?? null, current)) {
            return { conflict: true as const, server: current ? toSettingBlob(current) : null }
        }
        const updatedAt = deps.now()
        await deps.db.upsertSetting({
            userId: params.userId,
            name: params.name,
            format: input.format,
            content: input.content,
            size: input.content.length,
            updatedAt,
        })
        return { conflict: false as const, updated_at: updatedAt }
    },

    listKeys: async (userId: string) => {
        const rows = await deps.db.listSettings(userId)
        return rows.map(toSettingKey)
    },
})

export type SettingService = ReturnType<typeof createSettingService>
