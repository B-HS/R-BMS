import { and, eq } from 'drizzle-orm'
import type { Database } from '@server/db'
import { settingBlob } from '@server/db/schema'
import { createSettingService } from '@server/service/domain/setting/setting.service'

export const composeSetting = (db: Database) => {
    const settingService = createSettingService({
        now: () => Date.now(),
        db: {
            findSetting: async ({ userId, name }) => {
                const [row] = await db
                    .select({
                        name: settingBlob.name,
                        format: settingBlob.format,
                        content: settingBlob.content,
                        size: settingBlob.size,
                        updatedAt: settingBlob.updatedAt,
                    })
                    .from(settingBlob)
                    .where(and(eq(settingBlob.userId, userId), eq(settingBlob.name, name)))
                    .limit(1)
                return row ?? null
            },
            upsertSetting: async (row) => {
                await db
                    .insert(settingBlob)
                    .values(row)
                    .onDuplicateKeyUpdate({ set: { format: row.format, content: row.content, size: row.size, updatedAt: row.updatedAt } })
            },
            listSettings: async (userId) => {
                const rows = await db
                    .select({ name: settingBlob.name, format: settingBlob.format, size: settingBlob.size, updatedAt: settingBlob.updatedAt })
                    .from(settingBlob)
                    .where(eq(settingBlob.userId, userId))
                return rows
            },
        },
    })

    return { settingService }
}
