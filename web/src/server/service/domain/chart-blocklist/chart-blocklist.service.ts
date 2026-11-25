import { chartBlocklistSourceSchema } from '@server/dto/chart-blocklist'
import { CHART_BLOCKLIST_RAW_SOURCES } from '@server/service/domain/chart-blocklist/chart-blocklist.data'

export type ChartHashes = { md5?: string | null; sha256?: string | null }

/** The charts the server never ranks: a chart is blocked when its sha256 or its md5 is listed in any source. */
export const createChartBlocklist = (rawSources: readonly unknown[]) => {
    const sources = rawSources.map((raw) => chartBlocklistSourceSchema.parse(raw))
    const charts = sources.flatMap((source) => source.charts)
    const md5s = new Set(charts.map((chart) => chart.md5))
    const sha256s = new Set(charts.map((chart) => chart.sha256))
    return {
        sources,
        md5s: [...md5s],
        sha256s: [...sha256s],
        isBlocked: (chart: ChartHashes) => sha256s.has(chart.sha256 ?? '') || md5s.has(chart.md5 ?? ''),
    }
}

export type ChartBlocklist = ReturnType<typeof createChartBlocklist>

let loaded: ChartBlocklist | null = null

export const getChartBlocklist = () => {
    loaded ??= createChartBlocklist(CHART_BLOCKLIST_RAW_SOURCES)
    return loaded
}
