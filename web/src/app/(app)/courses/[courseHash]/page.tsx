import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { readCourseMeta } from '@server/service/public-reads'
import { CourseDetailContent } from '@widgets/course/course-detail-content'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

export const generateMetadata = async ({ params }: PageProps<'/courses/[courseHash]'>): Promise<Metadata> => {
    const { courseHash } = await params
    const [t, course] = await Promise.all([getTranslations('pages.route.courseDetail'), readCourseMeta(courseHash).catch(() => null)])
    if (!course) return buildPageMetadata({ title: t('title'), index: false })
    return buildPageMetadata({
        title: course.name || t('title'),
        description: t('metaDescription', { name: course.name || t('title') }),
        path: `/courses/${course.course_hash}`,
    })
}

const CourseDetailPage = async ({ params, searchParams }: PageProps<'/courses/[courseHash]'>) => {
    const t = await getTranslations('pages.route.courseDetail')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <CourseDetailContent params={params} searchParams={searchParams} />
        </Suspense>
    )
}

export default CourseDetailPage
