import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { buildApiUrl, fetchRaw } from '@shared/lib/fetch'
import type { CourseMeta } from '@entities/course.type'
import type { ScoreRecord } from '@entities/score.type'

export const courseMetaQueryOptions = (courseHash: string) =>
    queryOptions({
        queryKey: QUERY_KEY.COURSE.META(courseHash),
        queryFn: () => fetchRaw<CourseMeta>(buildApiUrl(`/courses/${encodeURIComponent(courseHash)}`)),
    })

export const courseRankingQueryOptions = (courseHash: string, params: { limit: number }) =>
    queryOptions({
        queryKey: QUERY_KEY.COURSE.RANKING(courseHash, params),
        queryFn: () => fetchRaw<ScoreRecord[]>(buildApiUrl(`/courses/${encodeURIComponent(courseHash)}/ranking`, params)),
    })

export const useCourseMeta = (courseHash: string) => useQuery(courseMetaQueryOptions(courseHash))

export const useCourseRanking = (courseHash: string, params: { limit: number }) => useQuery(courseRankingQueryOptions(courseHash, params))
