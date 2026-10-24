import type { SubmittedCourseRun } from '@server/service/domain/course/course.service'
import type { AcquiredDanRow, DanCourseRecord, DanDisplayRow, DanServiceDb, UserDanRow } from '@server/service/domain/dan/dan.service'

const keyOf = (row: { userId: string; system: string; label: string; assisted: boolean }) =>
    `${row.userId}\n${row.system}\n${row.label}\n${row.assisted}`

/** The dan tables held in memory: the dan courses and stored runs are given, the acquired dans and display choices are written by the service. */
export const createMemoryDanDb = (seed: { courses: DanCourseRecord[]; runs?: SubmittedCourseRun[]; courseNames?: Record<string, string> }) => {
    const acquired = new Map<string, UserDanRow>()
    const displays = new Map<string, DanDisplayRow>()
    const runs = seed.runs ?? []
    const counts = { listCourseScoresWithStages: 0 }

    const db: DanServiceDb = {
        findDanCourses: async (courseHashes) => seed.courses.filter((course) => courseHashes.includes(course.courseHash)),

        listDanCourseHashes: async () => seed.courses.map((course) => course.courseHash),

        listCourseScoresWithStages: async ({ courseHashes, after, limit }) => {
            counts.listCourseScoresWithStages += 1
            return runs
                .filter((run) => courseHashes.includes(run.courseHash) && run.stages !== null && run.userId !== null)
                .toSorted((left, right) => left.playedAt - right.playedAt || left.id.localeCompare(right.id))
                .filter((run) => !after || run.playedAt > after.playedAt || (run.playedAt === after.playedAt && run.id > after.id))
                .slice(0, limit)
        },

        insertUserDanIgnore: async (row) => {
            if (acquired.has(keyOf(row))) return false
            acquired.set(keyOf(row), row)
            return true
        },

        hasUserDan: async (params) => acquired.has(keyOf(params)),

        listUserDans: async (userIds): Promise<AcquiredDanRow[]> =>
            [...acquired.values()]
                .filter((row) => userIds.includes(row.userId))
                .map((row) => ({ ...row, courseName: seed.courseNames?.[row.courseHash] ?? '' })),

        listDisplays: async (userIds) => [...displays.values()].filter((row) => userIds.includes(row.userId)),

        upsertDisplay: async (row) => {
            displays.set(`${row.userId}\n${row.system}`, row)
        },

        deleteDisplay: async ({ userId, system }) => {
            displays.delete(`${userId}\n${system}`)
        },
    }

    return { db, acquired, displays, counts }
}
