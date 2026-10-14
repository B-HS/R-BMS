import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { CourseDetailPanel } from '@widgets/course/course-detail-panel'
import { courseMetaQueryOptions, courseRankingQueryOptions } from '@entities/course.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readBooleanParam } from '@shared/lib/search-params'
import { INCLUDE_SCRATCH_AUTO_PARAM } from '@shared/constants/scratch-auto'

const RANKING_LIMIT = 50

type CourseDetailContentProps = {
    params: Promise<{ courseHash: string }>
    searchParams: Promise<Record<string, string | string[] | undefined>>
}

export const CourseDetailContent = async ({ params, searchParams }: CourseDetailContentProps) => {
    const { courseHash } = await params
    const rankingParams = { limit: RANKING_LIMIT, include_scratch_auto: readBooleanParam(await searchParams, INCLUDE_SCRATCH_AUTO_PARAM) }

    const queryClient = getQueryClient()
    await Promise.all([
        fetchQueryOrNotFound(queryClient, courseMetaQueryOptions(courseHash)),
        queryClient.prefetchQuery(courseRankingQueryOptions(courseHash, rankingParams)),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <CourseDetailPanel courseHash={courseHash} rankingParams={rankingParams} />
        </HydrationBoundary>
    )
}
