import { and, asc, count, eq, gt, inArray, lt } from 'drizzle-orm'
import type { Database } from '@server/db'
import { chartPattern } from '@server/db/schema'
import { hasMysqlErrorCode, MYSQL_ERROR_CODE } from '@server/lib/mysql-error'
import { withLockRetry } from '@server/lib/with-lock-retry'
import { createChartPatternService, type ChartPatternServiceDb, type CoverageTable } from '@server/service/domain/chart-pattern/chart-pattern.service'

/** md5s asked about in one `IN` list, so a table of any size stays well under the statement's placeholder limit. */
const MD5_LOOKUP_CHUNK = 1000

const chunksOf = <T>(items: readonly T[], size: number) =>
    Array.from({ length: Math.ceil(items.length / size) }, (_, index) => items.slice(index * size, (index + 1) * size))

export const createChartPatternDb = (db: Database) =>
    ({
        findMeta: async (md5) => {
            const [row] = await db
                .select({
                    parserVersion: chartPattern.parserVersion,
                    contentHash: chartPattern.contentHash,
                    featuresVersion: chartPattern.featuresVersion,
                })
                .from(chartPattern)
                .where(eq(chartPattern.md5, md5))
                .limit(1)
            return row ?? null
        },
        insertIfAbsent: withLockRetry(async (row) => {
            try {
                await db.insert(chartPattern).values(row)
                return true
            } catch (error) {
                if (hasMysqlErrorCode(error, [MYSQL_ERROR_CODE.DUPLICATE_ENTRY])) return false
                throw error
            }
        }),
        replace: withLockRetry(async (row) => {
            const { md5, ...rest } = row
            await db.update(chartPattern).set(rest).where(eq(chartPattern.md5, md5))
        }),
        updateFeatures: withLockRetry(async ({ md5, features, featuresVersion, updatedAt }) => {
            await db.update(chartPattern).set({ features, featuresVersion, updatedAt }).where(eq(chartPattern.md5, md5))
        }),
        findStored: async (md5) => {
            const [row] = await db
                .select({ data: chartPattern.data, features: chartPattern.features, featuresVersion: chartPattern.featuresVersion })
                .from(chartPattern)
                .where(eq(chartPattern.md5, md5))
                .limit(1)
            return row ?? null
        },
        listFeatures: async (md5s) => {
            const found = await Promise.all(
                chunksOf(md5s, MD5_LOOKUP_CHUNK).map(async (chunk) =>
                    db
                        .select({ md5: chartPattern.md5, features: chartPattern.features, featuresVersion: chartPattern.featuresVersion })
                        .from(chartPattern)
                        .where(inArray(chartPattern.md5, chunk)),
                ),
            )
            return found.flat()
        },
        listPresentMd5s: async (md5s) => {
            const found = await Promise.all(
                chunksOf(md5s, MD5_LOOKUP_CHUNK).map(async (chunk) =>
                    db.select({ md5: chartPattern.md5 }).from(chartPattern).where(inArray(chartPattern.md5, chunk)),
                ),
            )
            return found.flat().map((row) => row.md5)
        },
        countAll: async () => {
            const [row] = await db.select({ value: count() }).from(chartPattern)
            return row?.value ?? 0
        },
        countFeaturesBelow: async (version) => {
            const [row] = await db.select({ value: count() }).from(chartPattern).where(lt(chartPattern.featuresVersion, version))
            return row?.value ?? 0
        },
        listStaleMd5s: async ({ version, after, limit }) => {
            const rows = await db
                .select({ md5: chartPattern.md5 })
                .from(chartPattern)
                .where(and(lt(chartPattern.featuresVersion, version), after === null ? undefined : gt(chartPattern.md5, after)))
                .orderBy(asc(chartPattern.md5))
                .limit(limit)
            return rows.map((row) => row.md5)
        },
    }) satisfies ChartPatternServiceDb

export const composeChartPattern = (db: Database, deps: { listCoverageTables: () => readonly CoverageTable[] }) => ({
    chartPatternService: createChartPatternService({
        db: createChartPatternDb(db),
        now: () => Date.now(),
        listCoverageTables: deps.listCoverageTables,
    }),
})
