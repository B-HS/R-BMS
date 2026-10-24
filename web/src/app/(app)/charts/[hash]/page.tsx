import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { DEFAULT_INCLUDE_SCRATCH_AUTO } from '@shared/constants/scratch-auto'
import { formatChartTitle, formatHash } from '@shared/lib/format'
import { buildPageMetadata } from '@shared/lib/metadata'
import { readChartLeaderboard } from '@server/service/public-reads'
import { ChartDetailContent } from '@widgets/chart/chart-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async ({ params }: PageProps<'/charts/[hash]'>): Promise<Metadata> => {
    const { hash } = await params
    const [t, tShared, leaderboard] = await Promise.all([
        getTranslations('pages.route.chart'),
        getTranslations('pages.shared'),
        readChartLeaderboard(hash, { include_scratch_auto: DEFAULT_INCLUDE_SCRATCH_AUTO }).catch(() => null),
    ])
    if (!leaderboard?.chart) return buildPageMetadata({ title: t('fallbackTitle', { hash: formatHash(hash) }), index: false })
    const title = formatChartTitle(leaderboard.chart, tShared('untitled'))
    return buildPageMetadata({
        title,
        description: t('metaDescription', { subject: `${title}${leaderboard.chart.artist ? ` · ${leaderboard.chart.artist}` : ''}` }),
        path: `/charts/${leaderboard.chart.sha256}`,
    })
}

const ChartDetailPage = async ({ params, searchParams }: PageProps<'/charts/[hash]'>) => {
    const t = await getTranslations('pages.route.chart')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <ChartDetailContent params={params} searchParams={searchParams} />
        </Suspense>
    )
}

export default ChartDetailPage
