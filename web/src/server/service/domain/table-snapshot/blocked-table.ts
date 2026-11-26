import { createHash } from 'node:crypto'
import blockedSp12 from '../../../../../data/chart-blocklist-sp12.json'
import { blockedExtraLevelSchema, type BlockedExtraLevel } from '@server/dto/chart-blocklist'
import type { ChartBlocklistVersion, ListedChart } from '@server/service/domain/chart-blocklist/chart-blocklist.service'
import type { SnapshotTable } from '@server/service/domain/table-snapshot/table-snapshot.data'

export const BLOCKED_TABLE_ID = 'blocked'
export const BLOCKED_TABLE_NAME = 'R-BMS Blocked Charts'
export const BLOCKED_TABLE_SYMBOL = ''

const ISO_DATE_LENGTH = 10
const CHART_FILE_EXTENSION = /\.[^./]+$/

/** Folders put before the per-source ones: the SP 12 charts of the list (a snapshot, see the chart blocklist ADR). */
export const BLOCKED_EXTRA_LEVELS: readonly BlockedExtraLevel[] = [blockedExtraLevelSchema.parse(blockedSp12)]

const byNumberAwareText = (a: string, b: string) => a.localeCompare(b, 'en', { numeric: true })

/** The title a blocked chart is shown under: its file name without the extension (the list keeps no title). */
export const blockedChartTitle = (path: string) => (path.split('/').at(-1) ?? path).replace(CHART_FILE_EXTENSION, '')

/**
 * The charts of `chart_blocklist` as a difficulty table with one level per source (its label), in source id order with numbers
 * compared as numbers, so a player can add `/tables/blocked` to see which of their charts the server never ranks. The extra levels come
 * first and keep only the charts the list still has, so a chart can be in its source's folder and in an extra one. Null when empty.
 */
export const toBlockedTable = (
    charts: readonly ListedChart[],
    version: ChartBlocklistVersion,
    extraLevels: readonly BlockedExtraLevel[] = BLOCKED_EXTRA_LEVELS,
): SnapshotTable | null => {
    if (charts.length === 0) return null
    const listedMd5s = new Set(charts.map((chart) => chart.md5))
    const entryOf = (md5: string, level: string, title: string) => ({
        md5,
        level,
        title,
        artist: '',
        url: '',
        url_diff: '',
        name_diff: '',
        comment: '',
        lr2_bmsid: '',
    })
    const extraEntries = extraLevels.flatMap((extra) =>
        extra.charts.filter((chart) => listedMd5s.has(chart.md5)).map((chart) => entryOf(chart.md5, extra.label, chart.title)),
    )
    const sorted = charts.toSorted((a, b) => byNumberAwareText(a.sourceId, b.sourceId) || byNumberAwareText(a.path, b.path))
    const entries = [...extraEntries, ...sorted.map((chart) => entryOf(chart.md5, chart.sourceLabel, blockedChartTitle(chart.path)))]
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
