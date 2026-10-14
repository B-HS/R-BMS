import { Suspense } from 'react'
import type { Metadata } from 'next'
import { chartLeaderboardQueryOptions } from '@entities/chart.query'
import { formatChartTitle, formatHash } from '@shared/lib/format'
import { buildPageMetadata } from '@shared/lib/metadata'
import { getQueryClient } from '@shared/utils/get-query-client'
import { ChartDetailContent } from '@widgets/chart/chart-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async ({ params }: PageProps<'/charts/[hash]'>): Promise<Metadata> => {
    const { hash } = await params
    const leaderboard = await getQueryClient()
        .fetchQuery(chartLeaderboardQueryOptions(hash, { include_scratch_auto: false }))
        .catch(() => null)
    if (!leaderboard?.chart) return buildPageMetadata({ title: `차트 ${formatHash(hash)}`, index: false })
    return buildPageMetadata({
        title: formatChartTitle(leaderboard.chart),
        description: `${formatChartTitle(leaderboard.chart)}${leaderboard.chart.artist ? ` · ${leaderboard.chart.artist}` : ''} 의 R-BMS 순위.`,
        path: `/charts/${leaderboard.chart.sha256}`,
    })
}

const ChartDetailPage = ({ params, searchParams }: PageProps<'/charts/[hash]'>) => (
    <Suspense fallback={<ShellPageLoading title='차트' />}>
        <ChartDetailContent params={params} searchParams={searchParams} />
    </Suspense>
)

export default ChartDetailPage
