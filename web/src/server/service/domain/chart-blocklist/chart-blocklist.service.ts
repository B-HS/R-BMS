import type { ChartBlocklistRegisterInput } from '@server/dto/chart-blocklist'
import { isMd5Hash, isSha256Hash } from '@server/dto/common'

export type ChartHashes = { md5?: string | null; sha256?: string | null }

export type ChartBlocklistRow = { sha256: string; md5: string; sourceId: string; sourceLabel: string; path: string; addedBy: string | null }

export type ListedChart = Pick<ChartBlocklistRow, 'md5' | 'sourceId' | 'sourceLabel' | 'path'>

/** What changes whenever a chart is added: the count and the time of the latest addition (ms, 0 for an empty list). */
export type ChartBlocklistVersion = { charts: number; lastAddedAt: number }

export type ChartBlocklistServiceDb = {
    isListed: (hashes: { sha256: string | null; md5: string | null }) => Promise<boolean>
    findListed: (hashes: { sha256s: string[]; md5s: string[] }) => Promise<{ sha256: string; md5: string }[]>
    /** Inserts the rows, skipping any whose sha256 or md5 is listed already, and answers how many it inserted. */
    insertNew: (rows: ChartBlocklistRow[]) => Promise<number>
    listSources: () => Promise<{ sourceId: string; sourceLabel: string; charts: number }[]>
    listHashes: () => Promise<{ sha256s: string[]; md5s: string[] }>
    listCharts: () => Promise<ListedChart[]>
    version: () => Promise<ChartBlocklistVersion>
}

/** The charts the server never ranks: a chart is blocked when its sha256 or its md5 is in `chart_blocklist`. */
export const createChartBlocklistService = (deps: { db: ChartBlocklistServiceDb }) => ({
    isBlocked: async (chart: ChartHashes) => {
        const sha256 = chart.sha256 && isSha256Hash(chart.sha256) ? chart.sha256 : null
        const md5 = chart.md5 && isMd5Hash(chart.md5) ? chart.md5 : null
        if (!sha256 && !md5) return false
        return deps.db.isListed({ sha256, md5 })
    },

    /** Adds the charts no list has yet (by sha256 or md5) under the source. Sending the same charts again adds nothing. */
    register: async (input: ChartBlocklistRegisterInput & { addedBy: string }) => {
        const listed = await deps.db.findListed({ sha256s: input.charts.map((chart) => chart.sha256), md5s: input.charts.map((chart) => chart.md5) })
        const listedSha256s = new Set(listed.map((row) => row.sha256))
        const listedMd5s = new Set(listed.map((row) => row.md5))
        const fresh = input.charts.filter((chart) => !listedSha256s.has(chart.sha256) && !listedMd5s.has(chart.md5))
        const rows = fresh.map((chart) => ({ ...chart, sourceId: input.source.id, sourceLabel: input.source.label, addedBy: input.addedBy }))
        const added = rows.length === 0 ? 0 : await deps.db.insertNew(rows)
        return { received: input.charts.length, added, alreadyListed: input.charts.length - added }
    },

    listSources: () => deps.db.listSources(),
    listHashes: () => deps.db.listHashes(),
    listCharts: () => deps.db.listCharts(),
    version: () => deps.db.version(),
})

export type ChartBlocklistService = ReturnType<typeof createChartBlocklistService>
