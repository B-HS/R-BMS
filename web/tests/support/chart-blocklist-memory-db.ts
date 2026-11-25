import type { ChartBlocklistRow, ChartBlocklistServiceDb } from '@server/service/domain/chart-blocklist/chart-blocklist.service'

/** An in-memory `chart_blocklist`: sha256 is the key and md5 is unique, as migration 0015 makes them. */
export const createMemoryChartBlocklistDb = (initial: ChartBlocklistRow[] = []) => {
    const rows = new Map(initial.map((row) => [row.sha256, row]))
    const calls = { isListed: 0 }
    const db: ChartBlocklistServiceDb = {
        isListed: async ({ sha256, md5 }) => {
            calls.isListed += 1
            return [...rows.values()].some((row) => row.sha256 === sha256 || row.md5 === md5)
        },
        findListed: async ({ sha256s, md5s }) =>
            [...rows.values()].filter((row) => sha256s.includes(row.sha256) || md5s.includes(row.md5)).map(({ sha256, md5 }) => ({ sha256, md5 })),
        insertNew: async (incoming) => {
            const md5s = new Set([...rows.values()].map((row) => row.md5))
            const fresh = incoming.filter((row) => !rows.has(row.sha256) && !md5s.has(row.md5))
            fresh.forEach((row) => rows.set(row.sha256, row))
            return fresh.length
        },
        listSources: async () =>
            [...Map.groupBy(rows.values(), (row) => row.sourceId).entries()]
                .toSorted(([a], [b]) => Number(a > b) - Number(a < b))
                .map(([sourceId, group]) => ({ sourceId, sourceLabel: group[0].sourceLabel, charts: group.length })),
        listHashes: async () => ({ sha256s: [...rows.keys()], md5s: [...rows.values()].map((row) => row.md5) }),
    }
    return { db, rows, calls }
}
