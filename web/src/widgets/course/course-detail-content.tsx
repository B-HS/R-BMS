import { dehydrate, HydrationBoundary } from '@tanstack/react-query'
import { CourseDetailPanel } from '@widgets/course/course-detail-panel'
import { courseMetaQueryOptions, courseRankingQueryOptions } from '@entities/course.query'
import { fetchQueryOrNotFound } from '@shared/lib/fetch-query-or-not-found'
import { getQueryClient } from '@shared/utils/get-query-client'
import { readIncludeScratchAutoParam } from '@shared/lib/scratch-auto-param'
import { rankingQuerySchema } from '@server/dto/score'
import { parseQueryParams, readCourseMeta, readCourseRanking } from '@server/service/public-reads'

const RANKING_LIMIT = 50

type CourseDetailContentProps = {
    params: Promise<{ courseHash: string }>
    searchParams: Promise<Record<string, string | string[] | undefined>>
}

export const CourseDetailContent = async ({ params, searchParams }: CourseDetailContentProps) => {
    const { courseHash } = await params
    const rankingParams = { limit: RANKING_LIMIT, include_scratch_auto: readIncludeScratchAutoParam(await searchParams) }

    const queryClient = getQueryClient()
    await Promise.all([
        fetchQueryOrNotFound(queryClient, { ...courseMetaQueryOptions(courseHash), queryFn: () => readCourseMeta(courseHash) }),
        queryClient.prefetchQuery({
            ...courseRankingQueryOptions(courseHash, rankingParams),
            queryFn: () => readCourseRanking(courseHash, parseQueryParams(rankingQuerySchema, rankingParams)),
        }),
    ])

    return (
        <HydrationBoundary state={dehydrate(queryClient)}>
            <CourseDetailPanel courseHash={courseHash} rankingParams={rankingParams} />
        </HydrationBoundary>
    )
}
