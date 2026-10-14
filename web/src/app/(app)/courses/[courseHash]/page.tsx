import { Suspense } from 'react'
import type { Metadata } from 'next'
import { buildPageMetadata } from '@shared/lib/metadata'
import { readCourseMeta } from '@server/service/public-reads'
import { CourseDetailContent } from '@widgets/course/course-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async ({ params }: PageProps<'/courses/[courseHash]'>): Promise<Metadata> => {
    const { courseHash } = await params
    const course = await readCourseMeta(courseHash).catch(() => null)
    if (!course) return buildPageMetadata({ title: '코스', index: false })
    return buildPageMetadata({
        title: course.name || '코스',
        description: `${course.name || '코스'} 의 구성 차트와 R-BMS 순위.`,
        path: `/courses/${course.course_hash}`,
    })
}

const CourseDetailPage = ({ params, searchParams }: PageProps<'/courses/[courseHash]'>) => (
    <Suspense fallback={<ShellPageLoading title='코스' />}>
        <CourseDetailContent params={params} searchParams={searchParams} />
    </Suspense>
)

export default CourseDetailPage
