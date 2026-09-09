import { Suspense } from 'react'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { CourseDetailContent } from '@widgets/course/course-detail-content'

const CourseDetailPage = ({ params }: { params: Promise<{ courseHash: string }> }) => (
    <Suspense fallback={<PanelSkeleton height='page' />}>
        <CourseDetailContent params={params} />
    </Suspense>
)

export default CourseDetailPage
