import { queryOptions, useQuery } from '@tanstack/react-query'
import { QUERY_KEY } from '@shared/constants/query-key'
import { ApiError, buildApiUrl, fetchEnvelope, fetchRaw } from '@shared/lib/fetch'
import type { CourseCatalog, CourseCatalogEntry, CourseMeta, CourseRankingParams } from '@entities/course.type'
import type { ScoreRecord } from '@entities/score.type'

const NOT_FOUND_STATUS = 404

export const courseMetaQueryOptions = (courseHash: string) =>
    queryOptions({
        queryKey: QUERY_KEY.COURSE.META(courseHash),
        queryFn: () => fetchRaw<CourseMeta>(buildApiUrl(`/courses/${encodeURIComponent(courseHash)}`)),
    })

export const courseRankingQueryOptions = (courseHash: string, params: CourseRankingParams) =>
    queryOptions({
        queryKey: QUERY_KEY.COURSE.RANKING(courseHash, params),
        queryFn: () => fetchRaw<ScoreRecord[]>(buildApiUrl(`/courses/${encodeURIComponent(courseHash)}/ranking`, params)),
    })

export const courseCatalogQueryOptions = () =>
    queryOptions({
        queryKey: QUERY_KEY.COURSE.CATALOG,
        queryFn: () => fetchEnvelope<CourseCatalog>(buildApiUrl('/fe/courses')),
    })

/** A course that is no part of the catalog (one a player brought along) has no entry: that answer is `null`, not an error. */
export const courseCatalogEntryQueryOptions = (courseHash: string) =>
    queryOptions({
        queryKey: QUERY_KEY.COURSE.CATALOG_ENTRY(courseHash),
        queryFn: async () => {
            try {
                return await fetchEnvelope<CourseCatalogEntry>(buildApiUrl(`/fe/courses/${encodeURIComponent(courseHash)}/catalog`))
            } catch (error) {
                if (error instanceof ApiError && error.status === NOT_FOUND_STATUS) return null
                throw error
            }
        },
    })

export const useCourseMeta = (courseHash: string) => useQuery(courseMetaQueryOptions(courseHash))

export const useCourseRanking = (courseHash: string, params: CourseRankingParams) => useQuery(courseRankingQueryOptions(courseHash, params))

export const useCourseCatalog = () => useQuery(courseCatalogQueryOptions())

export const useCourseCatalogEntry = (courseHash: string) => useQuery(courseCatalogEntryQueryOptions(courseHash))
