import type { CourseCatalogDanOutput, CourseCatalogEntryOutput, CourseCatalogOutput, CourseCatalogSourceOutput } from '@server/dto/course-catalog'
import type {
    CatalogChart,
    CatalogCourse,
    CatalogDan,
    CatalogDefinition,
    CatalogGroup,
    CatalogGroupPlan,
    CatalogSource,
    CatalogSyncPlan,
} from '@server/service/domain/course-catalog/course-catalog.type'
import { danSystemName } from '@server/service/domain/course-catalog/dan-of-course'

export type StoredCatalogCourse = {
    courseHash: string
    name: string
    lntype: number
    constraint: unknown
    trophy: unknown
    charts: CatalogChart[]
    dan: CatalogDan | null
}

export type StoredGroupMember = { courseHash: string; position: number; active: boolean }

export type CatalogGroupRow = Omit<CatalogGroup, 'courses'> & { source: CatalogSource }

export type CatalogMemberRow = StoredGroupMember & { groupId: string; name: string; stageCount: number; dan: CatalogDan | null }

export type CatalogEntryRow = {
    group: CatalogGroupRow
    active: boolean
    course: { courseHash: string; name: string }
    charts: CatalogChart[]
    dan: CatalogDan | null
}

export type CourseCatalogServiceDb = {
    findGroupMembers: (groupId: string) => Promise<StoredGroupMember[]>
    findCourses: (courseHashes: string[]) => Promise<StoredCatalogCourse[]>
    applyGroup: (params: {
        source: CatalogSource
        group: Omit<CatalogGroup, 'courses'>
        courses: CatalogCourse[]
        deactivate: string[]
    }) => Promise<void>
    listCatalog: () => Promise<{ groups: CatalogGroupRow[]; members: CatalogMemberRow[] }>
    findEntry: (courseHash: string) => Promise<CatalogEntryRow | null>
}

export type CourseCatalogServiceDeps = { db: CourseCatalogServiceDb }

type CourseStatus = 'inserted' | 'changed' | 'unchanged'

const sortKeys = (_key: string, value: unknown) =>
    typeof value === 'object' && value !== null && !Array.isArray(value)
        ? Object.fromEntries(Object.entries(value).toSorted(([a], [b]) => a.localeCompare(b)))
        : value

/** Whether two JSON values are the same, whatever order the database keeps the keys of an object in. */
const isSameJson = (stored: unknown, wanted: unknown) => JSON.stringify(stored, sortKeys) === JSON.stringify(wanted, sortKeys)

const isSameCourse = (stored: StoredCatalogCourse, course: CatalogCourse) =>
    stored.name === course.name &&
    stored.lntype === course.lntype &&
    isSameJson(stored.constraint, course.constraint) &&
    isSameJson(stored.trophy, course.trophy) &&
    isSameJson(stored.charts, course.charts) &&
    isSameJson(stored.dan, course.dan)

const toDanOutput = (dan: CatalogDan | null): CourseCatalogDanOutput | null =>
    dan ? { system: dan.system, system_name: danSystemName(dan.system), label: dan.label, rank: dan.rank } : null

const toSourceOutput = (source: CatalogSource): CourseCatalogSourceOutput => ({
    id: source.id,
    kind: source.kind,
    name: source.name,
    maker: source.maker,
    page_url: source.pageUrl,
    file_url: source.fileUrl,
    retrieved_at: source.retrievedAt,
    file_sha256: source.fileSha256,
    detail: source.detail,
})

export const createCourseCatalogService = (deps: CourseCatalogServiceDeps) => {
    const planGroup = async (group: CatalogGroup) => {
        const hashes = group.courses.map((course) => course.courseHash)
        const [members, stored] = await Promise.all([deps.db.findGroupMembers(group.id), deps.db.findCourses(hashes)])
        const memberOf = new Map(members.map((member) => [member.courseHash, member]))
        const storedOf = new Map(stored.map((course) => [course.courseHash, course]))
        const statuses = group.courses.map((course, position): CourseStatus => {
            const existing = storedOf.get(course.courseHash)
            if (!existing) return 'inserted'
            const member = memberOf.get(course.courseHash)
            return isSameCourse(existing, course) && member?.position === position && member.active ? 'unchanged' : 'changed'
        })
        const listed = new Set(hashes)
        const deactivate = members.filter((member) => member.active && !listed.has(member.courseHash)).map((member) => member.courseHash)
        const countOf = (status: CourseStatus) => statuses.filter((candidate) => candidate === status).length
        const plan: CatalogGroupPlan = {
            id: group.id,
            inserted: countOf('inserted'),
            changed: countOf('changed'),
            unchanged: countOf('unchanged'),
            deactivated: deactivate.length,
            danCourses: group.courses.filter((course) => course.dan !== null).length,
            newDanHashes: group.courses
                .filter((course) => course.dan !== null && !storedOf.get(course.courseHash)?.dan)
                .map((course) => course.courseHash),
        }
        return { plan, deactivate }
    }

    return {
        planSync: async (definition: CatalogDefinition): Promise<CatalogSyncPlan> => {
            const groups: CatalogGroupPlan[] = []
            for (const group of definition.groups) groups.push((await planGroup(group)).plan)
            return { groups }
        },

        applySync: async (definition: CatalogDefinition): Promise<CatalogSyncPlan> => {
            const groups: CatalogGroupPlan[] = []
            for (const { courses, ...head } of definition.groups) {
                const { plan, deactivate } = await planGroup({ ...head, courses })
                await deps.db.applyGroup({ source: definition.source, group: head, courses, deactivate })
                groups.push(plan)
            }
            return { groups }
        },

        listGroups: async (): Promise<CourseCatalogOutput> => {
            const { groups, members } = await deps.db.listCatalog()
            return {
                groups: groups
                    .toSorted((a, b) => a.position - b.position || a.id.localeCompare(b.id))
                    .map((group) => ({
                        id: group.id,
                        name: group.name,
                        keys: group.keys,
                        source: toSourceOutput(group.source),
                        courses: members
                            .filter((member) => member.groupId === group.id)
                            .toSorted((a, b) => a.position - b.position)
                            .map((member) => ({
                                course_hash: member.courseHash,
                                name: member.name,
                                stage_count: member.stageCount,
                                active: member.active,
                                dan: toDanOutput(member.dan),
                            })),
                    })),
            }
        },

        getEntry: async (courseHash: string): Promise<CourseCatalogEntryOutput | null> => {
            const entry = await deps.db.findEntry(courseHash)
            if (!entry) return null
            return {
                course_hash: entry.course.courseHash,
                name: entry.course.name,
                active: entry.active,
                group: { id: entry.group.id, name: entry.group.name, keys: entry.group.keys },
                source: toSourceOutput(entry.group.source),
                dan: toDanOutput(entry.dan),
                charts: entry.charts.map((chart, position) => ({ position, md5: chart.md5, sha256: chart.sha256, title: chart.title })),
            }
        },
    }
}

export type CourseCatalogService = ReturnType<typeof createCourseCatalogService>
