import { SETTING_NAMES_PER_USER_MAX, type SettingPutInput } from '@server/dto/setting'

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
    /**
     * Stores a blob and says whether it did. With a base stamp the write happens only while the stored stamp is still that base
     * (0 meaning no row yet), decided by the one statement that writes it, so two writers holding the same base can not both win.
     */
    writeSetting: (row: {
        userId: string
        name: string
        format: string
        content: string
        size: number
        updatedAt: number
        baseUpdatedAt: number | null
    }) => Promise<boolean>
    countSettings: (userId: string) => Promise<number>
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
        const baseUpdatedAt = input.base_updated_at ?? null
        const where = { userId: params.userId, name: params.name }
        const current = await deps.db.findSetting(where)
        if (isSettingConflict(baseUpdatedAt, current)) return { status: 'conflict' as const, server: current ? toSettingBlob(current) : null }
        if (!current && (await deps.db.countSettings(params.userId)) >= SETTING_NAMES_PER_USER_MAX) {
            return { status: 'too_many_names' as const, limit: SETTING_NAMES_PER_USER_MAX }
        }
        const updatedAt = Math.max(deps.now(), (current?.updatedAt ?? 0) + 1)
        const stored = await deps.db.writeSetting({
            ...where,
            format: input.format,
            content: input.content,
            size: input.content.length,
            updatedAt,
            baseUpdatedAt,
        })
        if (stored) return { status: 'stored' as const, updated_at: updatedAt }
        const latest = await deps.db.findSetting(where)
        return { status: 'conflict' as const, server: latest ? toSettingBlob(latest) : null }
    },

    listKeys: async (userId: string) => {
        const rows = await deps.db.listSettings(userId)
        return rows.map(toSettingKey)
    },
})

export type SettingService = ReturnType<typeof createSettingService>
