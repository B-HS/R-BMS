import { desc } from 'drizzle-orm'
import type { Database } from '@server/db'
import { clientBuild } from '@server/db/schema'
import { createBuildService } from '@server/service/domain/admin/build.service'

export const composeAdmin = (db: Database) => {
    const buildService = createBuildService({
        db: {
            listBuilds: async () =>
                db
                    .select({
                        sha256: clientBuild.sha256,
                        version: clientBuild.version,
                        platform: clientBuild.platform,
                        channel: clientBuild.channel,
                        releasedAt: clientBuild.releasedAt,
                        trusted: clientBuild.trusted,
                        note: clientBuild.note,
                    })
                    .from(clientBuild)
                    .orderBy(desc(clientBuild.releasedAt)),
            upsertBuild: async (row) => {
                await db
                    .insert(clientBuild)
                    .values(row)
                    .onDuplicateKeyUpdate({
                        set: {
                            version: row.version,
                            platform: row.platform,
                            channel: row.channel,
                            releasedAt: row.releasedAt,
                            trusted: row.trusted,
                            note: row.note,
                        },
                    })
            },
        },
    })

    return { buildService }
}
