import { and, eq, inArray } from 'drizzle-orm'
import type { Database } from '@server/db'
import { chart, chartBest, score } from '@server/db/schema'
import { getSnapshotTables } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { createTableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { chartBestScope } from '@server/compose/best-scope'

export const composeTableSnapshot = (db: Database) => {
    const tableSnapshotService = createTableSnapshotService({
        loadTables: getSnapshotTables,
        db: {
            listBestByMd5: async ({ userId, md5s }) => {
                if (md5s.length === 0) return []
                const rows = await db
                    .select({
                        md5: chart.md5,
                        clear: chartBest.clear,
                        assistGrade: chartBest.assistGrade,
                        exScore: chartBest.exScore,
                        minbp: chartBest.minbp,
                        maxCombo: chartBest.maxCombo,
                        totalNotes: score.notes,
                        playedAt: score.playedAt,
                        scratchAuto: chartBest.scratchAuto,
                    })
                    .from(chartBest)
                    .innerJoin(chart, eq(chart.sha256, chartBest.chartSha256))
                    .innerJoin(score, eq(score.id, chartBest.scoreId))
                    .where(and(eq(chartBest.userId, userId), chartBestScope(db, true), inArray(chart.md5, md5s)))
                return rows.flatMap(({ md5, ...record }) => (md5 ? [{ md5, ...record }] : []))
            },
        },
    })

    return { tableSnapshotService }
}
