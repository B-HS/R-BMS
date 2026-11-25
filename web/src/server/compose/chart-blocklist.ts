import { asc, count, eq, inArray, max, or, type SQL } from 'drizzle-orm'
import type { Database } from '@server/db'
import { chartBlocklist } from '@server/db/schema'
import { createChartBlocklistService, type ChartBlocklistServiceDb } from '@server/service/domain/chart-blocklist/chart-blocklist.service'

/** Hashes asked about in one `IN` list, so a list of any size stays well under the statement's placeholder limit. */
const HASH_LOOKUP_CHUNK = 1000

const chunksOf = <T>(items: readonly T[], size: number) =>
    Array.from({ length: Math.ceil(items.length / size) }, (_, index) => items.slice(index * size, (index + 1) * size))

export const createChartBlocklistDb = (db: Database) =>
    ({
        isListed: async ({ sha256, md5 }) => {
            const conditions = [sha256 ? eq(chartBlocklist.sha256, sha256) : null, md5 ? eq(chartBlocklist.md5, md5) : null].filter(
                (condition): condition is SQL => condition !== null,
            )
            const [row] = await db
                .select({ sha256: chartBlocklist.sha256 })
                .from(chartBlocklist)
                .where(or(...conditions))
                .limit(1)
            return Boolean(row)
        },

        findListed: async ({ sha256s, md5s }) => {
            const select = (where: SQL) => db.select({ sha256: chartBlocklist.sha256, md5: chartBlocklist.md5 }).from(chartBlocklist).where(where)
            const found = await Promise.all([
                ...chunksOf(sha256s, HASH_LOOKUP_CHUNK).map((chunk) => select(inArray(chartBlocklist.sha256, chunk))),
                ...chunksOf(md5s, HASH_LOOKUP_CHUNK).map((chunk) => select(inArray(chartBlocklist.md5, chunk))),
            ])
            return found.flat()
        },

        insertNew: async (rows) => {
            const [result] = await db.insert(chartBlocklist).ignore().values(rows)
            return result.affectedRows
        },

        listSources: async () => {
            const rows = await db
                .select({ sourceId: chartBlocklist.sourceId, sourceLabel: max(chartBlocklist.sourceLabel), charts: count() })
                .from(chartBlocklist)
                .groupBy(chartBlocklist.sourceId)
                .orderBy(asc(chartBlocklist.sourceId))
            return rows.map((row) => ({ ...row, sourceLabel: row.sourceLabel ?? '' }))
        },

        listHashes: async () => {
            const rows = await db.select({ sha256: chartBlocklist.sha256, md5: chartBlocklist.md5 }).from(chartBlocklist)
            return { sha256s: rows.map((row) => row.sha256), md5s: rows.map((row) => row.md5) }
        },
    }) satisfies ChartBlocklistServiceDb

export const composeChartBlocklist = (db: Database) => ({ chartBlocklistService: createChartBlocklistService({ db: createChartBlocklistDb(db) }) })
