import { z } from 'zod'
import type { CourseCatalogDanOutput, CourseCatalogEntryOutput, CourseCatalogOutput, CourseCatalogSourceOutput } from '@server/dto/course-catalog'

export type CourseChartRef = {
    md5: string
    sha256: string
}

export type CourseTrophy = {
    name: string
    scorerate: number
    smissrate: number
}

export type CourseRankingParams = {
    limit: number
    include_scratch_auto: boolean
}

export type CourseMeta = {
    course_hash: string
    name: string
    lntype: number
    charts: CourseChartRef[]
    constraint: string[]
    trophy: CourseTrophy[]
    extra: Record<string, unknown>
}

export type CourseCatalog = CourseCatalogOutput
export type CourseCatalogGroup = CourseCatalogOutput['groups'][number]
export type CourseCatalogCourse = CourseCatalogGroup['courses'][number]
export type CourseCatalogSource = CourseCatalogSourceOutput
export type CourseCatalogDan = CourseCatalogDanOutput
export type CourseCatalogEntry = CourseCatalogEntryOutput

const constraintSourceListSchema = z.array(z.object({ url: z.string(), retrieved_on: z.string() }))

/** Where the constraints and trophies of a bundled course set were read from; empty when the source names none. */
export const readConstraintSources = (source: CourseCatalogSource) => constraintSourceListSchema.catch([]).parse(source.detail.constraint_sources)
