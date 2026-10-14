import { and, count, eq } from 'drizzle-orm'
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
            writeSetting: async ({ baseUpdatedAt, ...row }) => {
                if (baseUpdatedAt === null) {
                    await db
                        .insert(settingBlob)
                        .values(row)
                        .onDuplicateKeyUpdate({ set: { format: row.format, content: row.content, size: row.size, updatedAt: row.updatedAt } })
                    return true
                }
                if (baseUpdatedAt === 0) {
                    const [inserted] = await db.insert(settingBlob).ignore().values(row)
                    return inserted.affectedRows > 0
                }
                const [replaced] = await db
                    .update(settingBlob)
                    .set({ format: row.format, content: row.content, size: row.size, updatedAt: row.updatedAt })
                    .where(and(eq(settingBlob.userId, row.userId), eq(settingBlob.name, row.name), eq(settingBlob.updatedAt, baseUpdatedAt)))
                return replaced.affectedRows > 0
            },
            countSettings: async (userId) => {
                const [row] = await db.select({ value: count() }).from(settingBlob).where(eq(settingBlob.userId, userId))
                return row?.value ?? 0
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
