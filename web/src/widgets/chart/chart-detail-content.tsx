import 'server-only'
import { notFound } from 'next/navigation'
import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { ChartLeaderboardPanel } from '@widgets/chart/chart-leaderboard-panel'
import { levelAnchorId } from '@widgets/table/table-snapshot-filter'
import { chartLeaderboardQueryOptions } from '@entities/chart.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { readIncludeScratchAutoParam } from '@shared/lib/scratch-auto-param'
import { getQueryClient } from '@shared/utils/get-query-client'
import { chartLeaderboardQuerySchema } from '@server/dto/fe'
import { parseQueryParams, readChartLeaderboard } from '@server/service/public-reads'
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

type ChartDetailContentProps = {
    params: Promise<{ hash: string }>
    searchParams: Promise<Record<string, string | string[] | undefined>>
}

export const ChartDetailContent = async ({ params, searchParams }: ChartDetailContentProps) => {
    const { hash } = await params
    const leaderboardParams = { include_scratch_auto: readIncludeScratchAutoParam(await searchParams) }
    const queryClient = getQueryClient()
    const leaderboard = await fetchQueryOrNotFound(queryClient, {
        ...chartLeaderboardQueryOptions(hash, leaderboardParams),
        queryFn: () => readChartLeaderboard(hash, parseQueryParams(chartLeaderboardQuerySchema, leaderboardParams)),
    })
    if (leaderboard && !leaderboard.chart) notFound()
    const md5 = leaderboard?.chart?.md5 || (MD5_PATTERN.test(hash) ? hash.toLowerCase() : '')

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <ChartLeaderboardPanel hash={hash} params={leaderboardParams} tableMemberships={md5 ? findTableMemberships(md5) : []} />
        </HydrationBoundary>
    )
}
