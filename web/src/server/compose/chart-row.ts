import type { chart } from '@server/db/schema'
import type { ChartRow } from '@server/service/domain/chart/chart.service'

export const toChartRow = (row: typeof chart.$inferSelect): ChartRow => ({
    sha256: row.sha256,
    md5: row.md5,
    title: row.title,
    subtitle: row.subtitle,
    genre: row.genre,
    artist: row.artist,
    subartist: row.subartist,
    level: row.level,
    total: row.total,
    mode: row.mode,
    lntype: row.lntype,
    judge: row.judge,
    minbpm: row.minbpm,
    maxbpm: row.maxbpm,
    notes: row.notes,
    hasLn: row.hasLn,
    hasCn: row.hasCn,
    hasHcn: row.hasHcn,
    hasMine: row.hasMine,
    hasRandom: row.hasRandom,
    hasStop: row.hasStop,
    url: row.url,
    appendurl: row.appendurl,
    extra: row.extra,
})
