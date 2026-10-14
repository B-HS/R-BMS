import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { ChartSearchContent } from '@widgets/search/chart-search-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({
    title: '차트 검색',
    description: 'R-BMS 점수 서버에 등록된 차트를 제목, 아티스트, 모드, 레벨로 찾습니다.',
    path: '/search',
})

const SearchPage = ({ searchParams }: PageProps<'/search'>) => (
    <ShellPage title='차트 검색' description='제목 · 아티스트 · 모드 · 레벨로 차트 찾기'>
        <Suspense fallback={<PanelSkeleton height='page' label='차트 검색 불러오는 중' />}>
            <ChartSearchContent searchParams={searchParams} />
        </Suspense>
    </ShellPage>
)

export default SearchPage
