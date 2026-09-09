import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ChartSearchContent } from '@widgets/search/chart-search-content'

const SearchPage = ({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <ChartSearchContent searchParams={searchParams} />
    </Suspense>
)

export default SearchPage
