import 'server-only'
import { notFound } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { ChartLeaderboardPanel } from '@widgets/chart/chart-leaderboard-panel'
import { levelAnchorId } from '@widgets/table/table-snapshot-filter'
import { chartLeaderboardQueryOptions } from '@entities/chart.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'
import { getSnapshotTables } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { summariseTable } from '@server/service/domain/table-snapshot/table-snapshot.service'

const MD5_PATTERN = /^[0-9a-f]{32}$/i

const findTableMemberships = (md5: string) =>
    [...getSnapshotTables().values()].flatMap((table) => {
        const summary = summariseTable(table)
        return table.entries
            .filter((entry) => entry.md5 === md5)
            .map((entry) => ({
                tableId: table.id,
                name: summary.name,
                symbol: summary.symbol,
                level: entry.level,
                href: `${summary.standard_paths.page}#${levelAnchorId(
                    entry.level,
                    summary.levels.findIndex((group) => group.level === entry.level),
                )}`,
            }))
    })

export const ChartDetailContent = async ({ params }: { params: Promise<{ hash: string }> }) => {
    const { hash } = await params
    const queryClient = getQueryClient()
    const leaderboard = await fetchQueryOrNotFound(queryClient, chartLeaderboardQueryOptions(hash))
    if (leaderboard && !leaderboard.chart) notFound()
    const md5 = leaderboard?.chart?.md5 || (MD5_PATTERN.test(hash) ? hash.toLowerCase() : '')

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <ChartLeaderboardPanel hash={hash} tableMemberships={md5 ? findTableMemberships(md5) : []} />
        </HydrationBoundary>
    )
}
