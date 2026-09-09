import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { CourseDetailPanel } from '@widgets/course/course-detail-panel'
import { courseMetaQueryOptions, courseRankingQueryOptions } from '@entities/course.query'
import { getQueryClient } from '@shared/utils/get-query-client'

const RANKING_PARAMS = { limit: 50 }

export const CourseDetailContent = async ({ params }: { params: Promise<{ courseHash: string }> }) => {
    const { courseHash } = await params

    const queryClient = getQueryClient()
    await Promise.all([
        queryClient.prefetchQuery(courseMetaQueryOptions(courseHash)),
        queryClient.prefetchQuery(courseRankingQueryOptions(courseHash, RANKING_PARAMS)),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <CourseDetailPanel courseHash={courseHash} rankingParams={RANKING_PARAMS} />
        </HydrationBoundary>
    )
}
