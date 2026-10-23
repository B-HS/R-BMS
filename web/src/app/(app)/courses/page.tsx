import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { buildPageMetadata } from '@shared/lib/metadata'
import { CourseCatalogContent } from '@widgets/course/course-catalog-content'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata = buildPageMetadata({
    title: '코스',
    description: 'R-BMS 에 등록된 GENOSIDE 2018 단위 코스와 Stella·Satellite 표 코스, 각 코스의 출처.',
    path: '/courses',
})

const CoursesPage = () => (
    <ShellPage title='코스' description='등록된 코스와 출처'>
        <Suspense fallback={<PanelSkeleton height='page' label='코스 불러오는 중' />}>
            <CourseCatalogContent />
        </Suspense>
    </ShellPage>
)

export default CoursesPage
