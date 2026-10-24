import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ChartSearchContent } from '@widgets/search/chart-search-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.search')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/search' })
}

const SearchPage = async ({ searchParams }: PageProps<'/search'>) => {
    const t = await getTranslations('pages.route.search')

    return (
        <ShellPage title={t('title')} description={t('description')}>
            <Suspense fallback={<PanelSkeleton height='page' label={t('loading')} />}>
                <ChartSearchContent searchParams={searchParams} />
            </Suspense>
        </ShellPage>
    )
}

export default SearchPage
