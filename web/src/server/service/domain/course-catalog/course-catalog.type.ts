import type { z } from 'zod'
import type { CourseSourceKind, DanSystemId } from '@server/dto/course-catalog'
import type { courseTrophySchema } from '@server/dto/course'

export type CourseTrophy = z.infer<typeof courseTrophySchema>

export type CatalogChart = { md5: string; sha256: string; title: string }

export type CatalogDan = { system: DanSystemId; label: string; rank: number }

export type CatalogCourse = {
    courseHash: string
    name: string
    lntype: number
    constraint: string[]
    trophy: CourseTrophy[]
    charts: CatalogChart[]
    dan: CatalogDan | null
}

export type CatalogSource = {
    id: string
    kind: CourseSourceKind
    name: string
    maker: string
    pageUrl: string
    fileUrl: string
    retrievedAt: number
    fileSha256: string
    detail: Record<string, unknown>
}

export type CatalogGroup = {
    id: string
    sourceId: string
    name: string
    keys: number
    position: number
    courses: CatalogCourse[]
}

export type CatalogDefinition = { source: CatalogSource; groups: CatalogGroup[] }

/** The part of a table's definition a table's courses are registered from. `TableSourceDefinition` of the table refresh satisfies it. */
export type TableCatalogSource = {
    id: string
    expectedName: string
    sourceName: string
    maker: string
    pageUrl: string
    headerUrl: string
    keys: number
    groupPosition: number
}

export type CatalogGroupPlan = {
    id: string
    inserted: number
    changed: number
    unchanged: number
    deactivated: number
    danCourses: number
    newDanHashes: string[]
}

export type CatalogSyncPlan = { groups: CatalogGroupPlan[] }
