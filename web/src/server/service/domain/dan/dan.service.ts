import type { userDan } from '@server/db/schema'
import type { DanSystemId } from '@server/dto/course-catalog'
import type { AcquiredDanOutput, DanDisplayInput, DanDisplayOutput, DisplayedDan, MyDansOutput } from '@server/dto/dan'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import type { SubmittedCourseRun } from '@server/service/domain/course/course.service'
import { DAN_SYSTEMS } from '@server/service/domain/course-catalog/dan-of-course'
import type { CatalogDan } from '@server/service/domain/course-catalog/course-catalog.type'
import {
    DAN_INELIGIBLE_REASON,
    judgeDanEligibility,
    type DanCourseFacts,
    type DanEligibility,
    type DanIneligibleReason,
} from '@server/service/domain/dan/dan-eligibility'

/** How many stored course runs a recomputation reads at a time. */
export const DAN_BACKFILL_PAGE = 500

export type UserDanRow = Omit<typeof userDan.$inferSelect, 'createdAt' | 'system'> & { system: DanSystemId }

export type AcquiredDanRow = UserDanRow & { courseName: string }

export type DanCourseRecord = DanCourseFacts & { courseHash: string }

export type DanDisplayRow = { userId: string; system: DanSystemId; label: string | null }

/** Where a recomputation stopped: runs are read oldest first, so the first run to earn a dan is the one stored. */
export type DanBackfillCursor = { playedAt: number; id: string }

export type DanServiceDb = {
    findDanCourses: (courseHashes: string[]) => Promise<DanCourseRecord[]>
    listDanCourseHashes: () => Promise<string[]>
    listCourseScoresWithStages: (params: { courseHashes: string[]; after: DanBackfillCursor | null; limit: number }) => Promise<SubmittedCourseRun[]>
    insertUserDanIgnore: (row: UserDanRow) => Promise<boolean>
    hasUserDan: (params: { userId: string; system: DanSystemId; label: string }) => Promise<boolean>
    listUserDans: (userIds: string[]) => Promise<AcquiredDanRow[]>
    listDisplays: (userIds: string[]) => Promise<DanDisplayRow[]>
    upsertDisplay: (row: DanDisplayRow) => Promise<void>
    deleteDisplay: (params: { userId: string; system: DanSystemId }) => Promise<void>
}

export type DanServiceDeps = { db: DanServiceDb }

/** `awarded` is the dan a run newly gave its player; `reason` is why the run gave none, and null when it was eligible (the player may already hold the dan). */
export type DanAwardResult = { awarded: CatalogDan | null; reason: DanIneligibleReason | null }

const toUserDanRow = (grant: Extract<DanEligibility, { eligible: true }>['grant'], run: SubmittedCourseRun): UserDanRow => ({
    userId: grant.userId,
    system: grant.dan.system,
    label: grant.dan.label,
    rank: grant.dan.rank,
    courseHash: run.courseHash,
    courseScoreId: run.id,
    courseGauge: grant.courseGauge,
    gauge: grant.gauge,
    acquiredAt: run.playedAt,
})

const toAcquiredOutput = (row: AcquiredDanRow): AcquiredDanOutput => ({
    label: row.label,
    rank: row.rank,
    course_hash: row.courseHash,
    course_name: row.courseName,
    course_gauge: row.courseGauge,
    acquired_at: row.acquiredAt,
})

const displayOf = (row: DanDisplayRow | undefined): DanDisplayOutput => {
    if (!row) return { mode: 'auto', label: null }
    if (row.label === null) return { mode: 'hidden', label: null }
    return { mode: 'label', label: row.label }
}

const shownLabelOf = (acquired: readonly AcquiredDanRow[], display: DanDisplayOutput) => {
    if (display.mode === 'hidden') return null
    if (display.mode === 'label') return acquired.find((row) => row.label === display.label)?.label ?? null
    return acquired.toSorted((left, right) => right.rank - left.rank)[0]?.label ?? null
}

const systemViewOf = (system: DanSystemId, rows: readonly AcquiredDanRow[], displays: readonly DanDisplayRow[]) => {
    const acquired = rows.filter((row) => row.system === system).toSorted((left, right) => left.rank - right.rank)
    const display = displayOf(displays.find((row) => row.system === system))
    return { acquired, display, shown: shownLabelOf(acquired, display) }
}

export const createDanService = (deps: DanServiceDeps) => {
    const listMine = async (userId: string): Promise<MyDansOutput> => {
        const [rows, displays] = await Promise.all([deps.db.listUserDans([userId]), deps.db.listDisplays([userId])])
        return {
            systems: DAN_SYSTEMS.map(({ id, name }) => {
                const { acquired, display, shown } = systemViewOf(id, rows, displays)
                return { system: id, name, display, shown, acquired: acquired.map(toAcquiredOutput) }
            }),
        }
    }

    return {
        awardFromRun: async (run: SubmittedCourseRun): Promise<DanAwardResult> => {
            const [course] = await deps.db.findDanCourses([run.courseHash])
            if (!course) return { awarded: null, reason: DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE }
            const verdict = judgeDanEligibility(run, course)
            if (!verdict.eligible) return { awarded: null, reason: verdict.reason }
            const isNew = await deps.db.insertUserDanIgnore(toUserDanRow(verdict.grant, run))
            return { awarded: isNew ? verdict.grant.dan : null, reason: null }
        },

        listDanCourseHashes: () => deps.db.listDanCourseHashes(),

        /**
         * Judges again every stored run of the given dan courses that has stage facts, oldest first, and gives the dans they earn.
         * `eligible` counts the runs that earn a dan, `awarded` the dans newly given (a player's first run to earn a dan keeps it,
         * later ones add nothing), and a dry run gives none. Runs from before the stage facts were stored are not read.
         */
        awardForCourses: async (params: { courseHashes: string[]; dryRun: boolean }) => {
            const courses = new Map((await deps.db.findDanCourses(params.courseHashes)).map((course) => [course.courseHash, course]))
            const courseHashes = [...courses.keys()]
            const totals = { scanned: 0, eligible: 0, awarded: 0 }
            if (courseHashes.length === 0) return totals
            let after: DanBackfillCursor | null = null
            for (;;) {
                const page = await deps.db.listCourseScoresWithStages({ courseHashes, after, limit: DAN_BACKFILL_PAGE })
                for (const run of page) {
                    totals.scanned += 1
                    const course = courses.get(run.courseHash)
                    const verdict = course ? judgeDanEligibility(run, course) : null
                    if (!verdict?.eligible) continue
                    totals.eligible += 1
                    if (!params.dryRun && (await deps.db.insertUserDanIgnore(toUserDanRow(verdict.grant, run)))) totals.awarded += 1
                }
                const last = page.at(-1)
                if (!last || page.length < DAN_BACKFILL_PAGE) return totals
                after = { playedAt: last.playedAt, id: last.id }
            }
        },

        listMine,

        setDisplay: async (userId: string, input: DanDisplayInput) => {
            if (input.mode === 'label' && !(await deps.db.hasUserDan({ userId, system: input.system, label: input.label })))
                throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'label' })
            if (input.mode === 'auto') await deps.db.deleteDisplay({ userId, system: input.system })
            else await deps.db.upsertDisplay({ userId, system: input.system, label: input.mode === 'label' ? input.label : null })
            return listMine(userId)
        },

        listDisplayed: async (userIds: string[]) => {
            if (userIds.length === 0) return new Map<string, DisplayedDan[]>()
            const [rows, displays] = await Promise.all([deps.db.listUserDans(userIds), deps.db.listDisplays(userIds)])
            return new Map(
                userIds.map((userId): [string, DisplayedDan[]] => {
                    const rowsOfUser = rows.filter((row) => row.userId === userId)
                    const displaysOfUser = displays.filter((row) => row.userId === userId)
                    return [
                        userId,
                        DAN_SYSTEMS.flatMap(({ id, name }) => {
                            const { shown } = systemViewOf(id, rowsOfUser, displaysOfUser)
                            return shown === null ? [] : [{ system: id, name, label: shown }]
                        }),
                    ]
                }),
            )
        },
    }
}

export type DanService = ReturnType<typeof createDanService>
