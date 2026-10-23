import type { CatalogGroup, CatalogSource } from '@server/service/domain/course-catalog/course-catalog.type'
import type {
    CatalogGroupRow,
    CatalogMemberRow,
    CourseCatalogServiceDb,
    StoredCatalogCourse,
    StoredGroupMember,
} from '@server/service/domain/course-catalog/course-catalog.service'

const withReversedKeys = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(withReversedKeys)
    if (typeof value === 'object' && value !== null)
        return Object.fromEntries(
            Object.entries(value)
                .reverse()
                .map(([key, nested]) => [key, withReversedKeys(nested)]),
        )
    return value
}

/**
 * The catalog tables held in memory. A JSON column comes back with its object keys in another order than they were written in
 * (MySQL keeps them sorted by length), which `withReversedKeys` does to the constraint and trophy a course is stored with.
 */
export const createMemoryCourseCatalogDb = () => {
    const sources = new Map<string, CatalogSource>()
    const groups = new Map<string, Omit<CatalogGroup, 'courses'>>()
    const courses = new Map<string, StoredCatalogCourse>()
    const members = new Map<string, StoredGroupMember & { groupId: string }>()
    const counts = { applyGroup: 0 }

    const memberKey = (groupId: string, courseHash: string) => `${groupId}\n${courseHash}`

    const groupRowOf = (group: Omit<CatalogGroup, 'courses'>): CatalogGroupRow[] => {
        const source = sources.get(group.sourceId)
        return source ? [{ ...group, source }] : []
    }

    const db: CourseCatalogServiceDb = {
        findGroupMembers: async (groupId) =>
            [...members.values()]
                .filter((member) => member.groupId === groupId)
                .map(({ courseHash, position, active }) => ({ courseHash, position, active })),

        findCourses: async (courseHashes) => courseHashes.flatMap((courseHash) => courses.get(courseHash) ?? []),

        applyGroup: async ({ source, group, courses: listed, deactivate }) => {
            counts.applyGroup += 1
            sources.set(source.id, source)
            groups.set(group.id, group)
            for (const [position, course] of listed.entries()) {
                courses.set(course.courseHash, {
                    courseHash: course.courseHash,
                    name: course.name,
                    lntype: course.lntype,
                    constraint: withReversedKeys(course.constraint),
                    trophy: withReversedKeys(course.trophy),
                    charts: course.charts,
                    dan: course.dan,
                })
                members.set(memberKey(group.id, course.courseHash), { groupId: group.id, courseHash: course.courseHash, position, active: true })
            }
            for (const courseHash of deactivate) {
                const member = members.get(memberKey(group.id, courseHash))
                if (member) members.set(memberKey(group.id, courseHash), { ...member, active: false })
            }
        },

        listCatalog: async () => ({
            groups: [...groups.values()].flatMap(groupRowOf),
            members: [...members.values()].flatMap((member): CatalogMemberRow[] => {
                const course = courses.get(member.courseHash)
                return course ? [{ ...member, name: course.name, stageCount: course.charts.length, dan: course.dan }] : []
            }),
        }),

        findEntry: async (courseHash) => {
            const member = [...members.values()].find((candidate) => candidate.courseHash === courseHash)
            const course = courses.get(courseHash)
            const group = member ? groups.get(member.groupId) : undefined
            const [groupRow] = group ? groupRowOf(group) : []
            return member && course && groupRow
                ? { group: groupRow, active: member.active, course: { courseHash, name: course.name }, charts: course.charts, dan: course.dan }
                : null
        },
    }

    return { db, counts, courses, members }
}
