import { createHash } from 'node:crypto'
import type { ChartBlocklistVersion, ListedChart } from '@server/service/domain/chart-blocklist/chart-blocklist.service'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'

export const BLOCKED_TABLE_ID = 'blocked'
export const BLOCKED_TABLE_NAME = 'R-BMS Blocked Charts'
export const BLOCKED_TABLE_SYMBOL = ''

const ISO_DATE_LENGTH = 10
const CHART_FILE_EXTENSION = /\.[^./]+$/

const byNumberAwareText = (a: string, b: string) => a.localeCompare(b, 'en', { numeric: true })

/** The title a blocked chart is shown under: its file name without the extension (the list keeps no title). */
export const blockedChartTitle = (path: string) => (path.split('/').at(-1) ?? path).replace(CHART_FILE_EXTENSION, '')

/**
 * The charts of `chart_blocklist` as a difficulty table with one level per source (its label), in source id order with numbers
 * compared as numbers, so a player can add `/tables/blocked` to see which of their charts the server never ranks. Null when empty.
 */
export const toBlockedTable = (charts: readonly ListedChart[], version: ChartBlocklistVersion): SnapshotTable | null => {
    if (charts.length === 0) return null
    const sorted = charts.toSorted((a, b) => byNumberAwareText(a.sourceId, b.sourceId) || byNumberAwareText(a.path, b.path))
    const entries = sorted.map((chart) => ({
        md5: chart.md5,
        level: chart.sourceLabel,
        title: blockedChartTitle(chart.path),
        artist: '',
        url: '',
        url_diff: '',
        name_diff: '',
        comment: '',
        lr2_bmsid: '',
    }))
    const levelOrder = [...new Set(entries.map((entry) => entry.level))]
    const text = JSON.stringify(entries)
    const updatedAt = new Date(version.lastAddedAt).toISOString()
    return {
        id: BLOCKED_TABLE_ID,
        header: {
            name: BLOCKED_TABLE_NAME,
            symbol: BLOCKED_TABLE_SYMBOL,
            data_url: '',
            level_order: levelOrder,
            last_update: updatedAt.slice(0, ISO_DATE_LENGTH),
        },
        entries,
        standardData: () => text,
        source: {
            origin: { page_url: '', header_url: '' },
            fetched_at: updatedAt,
            snapshot: {
                header_file_sha256: '',
                data_file_sha256: createHash('sha256').update(text).digest('hex'),
                entries: entries.length,
                levels: levelOrder.length,
            },
        },
        attribution: null,
        course: null,
    }
}

/** The blocked table, built again only when the list's version moved since this instance last built it. */
export const createBlockedTableCache = (blocklist: { listCharts: () => Promise<ListedChart[]>; version: () => Promise<ChartBlocklistVersion> }) => {
    let remembered: { version: ChartBlocklistVersion; table: SnapshotTable | null } | null = null
    return {
        load: async () => {
            const version = await blocklist.version()
            const isCurrent = remembered && remembered.version.charts === version.charts && remembered.version.lastAddedAt === version.lastAddedAt
            if (remembered && isCurrent) return remembered.table
            const table = toBlockedTable(await blocklist.listCharts(), version)
            remembered = { version, table }
            return table
        },
    }
}
