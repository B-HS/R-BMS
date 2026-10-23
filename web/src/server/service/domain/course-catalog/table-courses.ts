import { z } from 'zod'
import { columnTextSchema, isMd5Hash, isSha256Hash } from '@server/dto/common'
import { CATALOG_NAME_MAX_LENGTH } from '@server/dto/course-catalog'
import { canonicalConstraints, lntypeOfConstraints } from '@server/service/domain/course-catalog/constraint-tokens'
import { asciiLowercase, courseHashOf } from '@server/service/domain/course-catalog/course-hash'
import type { CatalogChart, CatalogCourse, CatalogDefinition } from '@server/service/domain/course-catalog/course-catalog.type'
import { danOfCourse } from '@server/service/domain/course-catalog/dan-of-course'
import type { TableSourceDefinition } from '@server/service/domain/table-refresh/table-sources'

export const DEFAULT_COURSE_NAME = 'No Course Title'

const TROPHY_MISSRATE_FLOOR = 0
const TROPHY_SCORERATE_CEILING = 100

const chartTitleSchema = columnTextSchema(CATALOG_NAME_MAX_LENGTH)

const chartObjectSchema = z.object({
    md5: z.string().default(''),
    sha256: z.string().default(''),
    title: z.string().default(''),
})

const trophyRuleSchema = z.object({
    name: z.string().default(''),
    missrate: z.number().default(0),
    scorerate: z.number().default(0),
})

/** A course entry of a table header as the client reads it (`Course`): the aliases of the reference implementation (`hash` and `song` for `charts`, `constraints`, `trophies`) are read too. */
const courseEntrySchema = z.object({
    name: z.string().default(''),
    charts: z.array(chartObjectSchema).optional(),
    hash: z.array(chartObjectSchema).optional(),
    song: z.array(chartObjectSchema).optional(),
    constraint: z.array(z.string()).optional(),
    constraints: z.array(z.string()).optional(),
    trophy: z.array(trophyRuleSchema).optional(),
    trophies: z.array(trophyRuleSchema).optional(),
})

const isObjectEntry = (entry: unknown): entry is Record<string, unknown> => typeof entry === 'object' && entry !== null && !Array.isArray(entry)

const hashText = (value: unknown) => {
    if (typeof value === 'string') return value
    return typeof value === 'number' ? String(value) : ''
}

const hashesOf = (value: unknown) => {
    if (value === undefined) return []
    return Array.isArray(value) ? value.map(hashText) : [hashText(value)]
}

const listedEntries = (headerCourse: unknown) =>
    Array.isArray(headerCourse) ? headerCourse.flatMap((item): unknown[] => (Array.isArray(item) ? item : [item])) : [headerCourse]

const chartsOfEntry = (entry: Record<string, unknown>, listed: z.infer<typeof courseEntrySchema>) => {
    const objectCharts = listed.charts ?? listed.hash ?? listed.song ?? []
    if (objectCharts.length > 0) return objectCharts
    const md5s = hashesOf(entry.md5)
    const sha256s = hashesOf(entry.sha256)
    return Array.from({ length: Math.max(md5s.length, sha256s.length) }, (_, stage) => ({
        md5: md5s[stage] ?? '',
        sha256: sha256s[stage] ?? '',
        title: '',
    }))
}

type ReadCourse = Omit<CatalogCourse, 'dan'>

const isStorableChart = (chart: CatalogChart) =>
    (chart.md5 !== '' || chart.sha256 !== '') && (chart.md5 === '' || isMd5Hash(chart.md5)) && (chart.sha256 === '' || isSha256Hash(chart.sha256))

const readCourse = (entry: unknown, titleOfMd5: ReadonlyMap<string, string>): ReadCourse | null => {
    if (!isObjectEntry(entry)) return null
    const listed = courseEntrySchema.safeParse(entry)
    if (!listed.success) return null
    const charts: CatalogChart[] = chartsOfEntry(entry, listed.data).map((chart) => {
        const md5 = asciiLowercase(chart.md5)
        return { md5, sha256: asciiLowercase(chart.sha256), title: chartTitleSchema.parse(chart.title || (titleOfMd5.get(md5) ?? '')) }
    })
    if (charts.length === 0 || !charts.every(isStorableChart)) return null
    const name = listed.data.name === '' ? DEFAULT_COURSE_NAME : listed.data.name
    if ([...name].length > CATALOG_NAME_MAX_LENGTH) return null
    const constraint = canonicalConstraints(listed.data.constraint ?? listed.data.constraints ?? [])
    const trophyRules = listed.data.trophy ?? listed.data.trophies ?? []
    return {
        courseHash: courseHashOf({ name, charts }),
        name,
        lntype: lntypeOfConstraints(constraint),
        constraint,
        trophy: trophyRules
            .filter((rule) => rule.name !== '' && rule.missrate > TROPHY_MISSRATE_FLOOR && rule.scorerate < TROPHY_SCORERATE_CEILING)
            .map((rule) => ({ name: rule.name, scorerate: rule.scorerate, smissrate: rule.missrate })),
        charts,
    }
}

/**
 * The courses a difficulty table header lists, as a catalog definition of one group, with how many entries were left out.
 *
 * It reads the `course` field the way the client does (`courses_from_table_header`), so a course has the id the client gives it:
 * a list of groups of courses, a flat list or one course object; chart hashes from a `charts` list or from `md5` and `sha256`
 * lists, lowercased. An entry is left out and counted when it is no object, can not be read as a course, has no chart, has a chart
 * with neither hash or with a hash or a name the database can not hold (a hash that is not full-length hex, a name past its column),
 * or has an id an earlier course already took. A chart's title is the one the course gives it, else the title
 * of the table entry with the same MD5. A header without a `course` field lists no course.
 */
export const tableCatalog = (params: {
    table: TableSourceDefinition
    headerCourse: unknown
    entries: readonly { md5: string; title: string }[]
    headerSha256: string
    dataSha256: string
    dataUrl: string
    fetchedAt: number
}): { definition: CatalogDefinition; skipped: number } => {
    const { table } = params
    const titleOfMd5 = new Map<string, string>()
    for (const entry of params.entries) {
        const md5 = asciiLowercase(entry.md5)
        if (!titleOfMd5.has(md5)) titleOfMd5.set(md5, entry.title)
    }
    const entries = params.headerCourse === undefined ? [] : listedEntries(params.headerCourse)
    const takenHashes = new Set<string>()
    const courses: CatalogCourse[] = []
    for (const entry of entries) {
        const read = readCourse(entry, titleOfMd5)
        if (!read || takenHashes.has(read.courseHash)) continue
        takenHashes.add(read.courseHash)
        courses.push({ ...read, dan: danOfCourse(table.id, read.name, courses.length) })
    }
    return {
        definition: {
            source: {
                id: table.id,
                kind: 'table',
                name: table.sourceName,
                maker: table.maker,
                pageUrl: table.pageUrl,
                fileUrl: table.headerUrl,
                retrievedAt: params.fetchedAt,
                fileSha256: params.headerSha256,
                detail: { data_url: params.dataUrl, data_sha256: params.dataSha256 },
            },
            groups: [{ id: table.id, sourceId: table.id, name: table.expectedName, keys: table.keys, position: table.groupPosition, courses }],
        },
        skipped: entries.length - courses.length,
    }
}
