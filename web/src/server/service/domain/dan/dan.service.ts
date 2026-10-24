import type { userDan } from '@server/db/schema'
import type { DanSystemId } from '@server/dto/course-catalog'
import {
    danDisplayLabel,
    type AcquiredDanOutput,
    type DanDisplayInput,
    type DanDisplayOutput,
    type DisplayedDan,
    type MyDansOutput,
} from '@server/dto/dan'
import { createAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import type { SubmittedCourseRun } from '@server/service/domain/course/course.service'
import { DAN_SYSTEMS } from '@server/service/domain/course-catalog/dan-of-course'
import type { CatalogDan } from '@server/service/domain/course-catalog/course-catalog.type'
import {
    DAN_AWARD_KIND,
    DAN_INELIGIBLE_REASON,
    judgeDanEligibility,
    type DanAwardKind,
    type DanCourseFacts,
    type DanEligibility,
    type DanIneligibleReason,
} from '@server/service/domain/dan/dan-eligibility'

/** How many stored course runs a recomputation reads at a time. */
export const DAN_BACKFILL_PAGE = 500

export type UserDanRow = Omit<typeof userDan.$inferSelect, 'createdAt' | 'system'> & { system: DanSystemId }

export type AcquiredDanRow = UserDanRow & { courseName: string }

export type DanCourseRecord = DanCourseFacts & { courseHash: string }

/** A player's display choice for one system: `label` null hides the system's dans, otherwise the dan of that label and kind is shown. */
export type DanDisplayRow = { userId: string; system: DanSystemId; label: string | null; assisted: boolean }

/** Where a recomputation stopped: runs are read oldest first, so the first run to earn a dan is the one stored. */
export type DanBackfillCursor = { playedAt: number; id: string }

export type DanServiceDb = {
    findDanCourses: (courseHashes: string[]) => Promise<DanCourseRecord[]>
    listDanCourseHashes: () => Promise<string[]>
    listCourseScoresWithStages: (params: { courseHashes: string[]; after: DanBackfillCursor | null; limit: number }) => Promise<SubmittedCourseRun[]>
    insertUserDanIgnore: (row: UserDanRow) => Promise<boolean>
    hasUserDan: (params: { userId: string; system: DanSystemId; label: string; assisted: boolean }) => Promise<boolean>
    listUserDans: (userIds: string[]) => Promise<AcquiredDanRow[]>
    listDisplays: (userIds: string[]) => Promise<DanDisplayRow[]>
    upsertDisplay: (row: DanDisplayRow) => Promise<void>
    deleteDisplay: (params: { userId: string; system: DanSystemId }) => Promise<void>
}

export type DanServiceDeps = { db: DanServiceDb }

/**
 * `awarded` is the dan a run newly gave its player, of either kind; `kind` is the kind of dan the run earns, and null when it earns
 * none; `reason` is why the run gave none, and null when it was eligible (the player may already hold the dan).
 */
export type DanAwardResult = { awarded: CatalogDan | null; kind: DanAwardKind | null; reason: DanIneligibleReason | null }

type EligibleVerdict = Extract<DanEligibility, { eligible: true }>

const toUserDanRow = ({ grant, kind }: EligibleVerdict, run: SubmittedCourseRun): UserDanRow => ({
    userId: grant.userId,
    system: grant.dan.system,
    label: grant.dan.label,
    assisted: kind === DAN_AWARD_KIND.ASSISTED,
    rank: grant.dan.rank,
    courseHash: run.courseHash,
    courseScoreId: run.id,
    courseGauge: grant.courseGauge,
    gauge: grant.gauge,
    acquiredAt: run.playedAt,
})

const toAcquiredOutput = (row: AcquiredDanRow): AcquiredDanOutput => ({
    label: row.label,
    assisted: row.assisted,
    rank: row.rank,
    course_hash: row.courseHash,
    course_name: row.courseName,
    course_gauge: row.courseGauge,
    acquired_at: row.acquiredAt,
})

const displayOf = (row: DanDisplayRow | undefined): DanDisplayOutput => {
    if (!row) return { mode: 'auto', label: null, assisted: false }
    if (row.label === null) return { mode: 'hidden', label: null, assisted: false }
    return { mode: 'label', label: row.label, assisted: row.assisted }
}

type HeldDan = Pick<AcquiredDanRow, 'label' | 'rank' | 'assisted'>

const highestOf = <T extends HeldDan>(held: readonly T[], assisted: boolean) =>
    held.filter((row) => row.assisted === assisted).toSorted((left, right) => right.rank - left.rank)[0]

/**
 * The dans of one system others see: none when hidden, the chosen one, or in auto the highest regular dan and also the highest
 * A-dan when it ranks above it (an A-dan alone when the player holds no regular dan of the system).
 */
const shownDansOf = <T extends HeldDan>(held: readonly T[], display: DanDisplayOutput) => {
    if (display.mode === 'hidden') return []
    if (display.mode === 'label') return held.filter((row) => row.label === display.label && row.assisted === display.assisted).slice(0, 1)
    const regular = highestOf(held, false)
    const assisted = highestOf(held, true)
    if (!assisted || (regular && regular.rank >= assisted.rank)) return regular ? [regular] : []
    return regular ? [regular, assisted] : [assisted]
}

const systemViewOf = (system: { id: DanSystemId; name: string }, rows: readonly AcquiredDanRow[], displays: readonly DanDisplayRow[]) => {
    const acquired = rows
        .filter((row) => row.system === system.id)
        .toSorted((left, right) => left.rank - right.rank || Number(left.assisted) - Number(right.assisted))
    const display = displayOf(displays.find((row) => row.system === system.id))
    const shown = shownDansOf(acquired, display).map((row): DisplayedDan => ({
        system: system.id,
        name: system.name,
        label: danDisplayLabel(row),
        assisted: row.assisted,
    }))
    return { acquired, display, shown }
}

export const createDanService = (deps: DanServiceDeps) => {
    const listMine = async (userId: string): Promise<MyDansOutput> => {
        const [rows, displays] = await Promise.all([deps.db.listUserDans([userId]), deps.db.listDisplays([userId])])
        return {
            systems: DAN_SYSTEMS.map((system) => {
                const { acquired, display, shown } = systemViewOf(system, rows, displays)
                return { system: system.id, name: system.name, display, shown, acquired: acquired.map(toAcquiredOutput) }
            }),
        }
    }

    return {
        awardFromRun: async (run: SubmittedCourseRun): Promise<DanAwardResult> => {
            const [course] = await deps.db.findDanCourses([run.courseHash])
            if (!course) return { awarded: null, kind: null, reason: DAN_INELIGIBLE_REASON.NOT_A_DAN_COURSE }
            const verdict = judgeDanEligibility(run, course)
            if (!verdict.eligible) return { awarded: null, kind: null, reason: verdict.reason }
            const isNew = await deps.db.insertUserDanIgnore(toUserDanRow(verdict, run))
            return { awarded: isNew ? verdict.grant.dan : null, kind: verdict.kind, reason: null }
        },

        listDanCourseHashes: () => deps.db.listDanCourseHashes(),

        /**
         * Judges again every stored run of the given dan courses that has stage facts, oldest first, and gives the dans they earn.
         * `eligible` counts the runs that earn a regular dan and `awarded` the regular dans newly given (a player's first run to earn
         * a dan keeps it, later ones add nothing); `eligibleAssisted` and `awardedAssisted` count the same for A-dans, and
         * `assistedLegacy` the runs that earn an A-dan only because a run stored without the server's answer to whether it would
         * have been ranked without assist is taken to have been. A dry run gives none. Runs from before the stage facts were stored
         * are not read.
         */
        awardForCourses: async (params: { courseHashes: string[]; dryRun: boolean }) => {
            const courses = new Map((await deps.db.findDanCourses(params.courseHashes)).map((course) => [course.courseHash, course]))
            const courseHashes = [...courses.keys()]
            const totals = { scanned: 0, eligible: 0, awarded: 0, eligibleAssisted: 0, awardedAssisted: 0, assistedLegacy: 0 }
            if (courseHashes.length === 0) return totals
            let after: DanBackfillCursor | null = null
            for (;;) {
                const page = await deps.db.listCourseScoresWithStages({ courseHashes, after, limit: DAN_BACKFILL_PAGE })
                for (const run of page) {
                    totals.scanned += 1
                    const course = courses.get(run.courseHash)
                    const verdict = course ? judgeDanEligibility(run, course) : null
                    if (!verdict?.eligible) continue
                    const isAssisted = verdict.kind === DAN_AWARD_KIND.ASSISTED
                    if (isAssisted) totals.eligibleAssisted += 1
                    else totals.eligible += 1
                    if (isAssisted && !run.ranked && run.rankedWithoutAssist === null) totals.assistedLegacy += 1
                    if (params.dryRun || !(await deps.db.insertUserDanIgnore(toUserDanRow(verdict, run)))) continue
                    if (isAssisted) totals.awardedAssisted += 1
                    else totals.awarded += 1
                }
                const last = page.at(-1)
                if (!last || page.length < DAN_BACKFILL_PAGE) return totals
                after = { playedAt: last.playedAt, id: last.id }
            }
        },

        listMine,

        setDisplay: async (userId: string, input: DanDisplayInput) => {
            if (input.mode === 'label' && !(await deps.db.hasUserDan({ userId, system: input.system, label: input.label, assisted: input.assisted })))
                throw createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'label' })
            if (input.mode === 'auto') await deps.db.deleteDisplay({ userId, system: input.system })
            else if (input.mode === 'hidden') await deps.db.upsertDisplay({ userId, system: input.system, label: null, assisted: false })
            else await deps.db.upsertDisplay({ userId, system: input.system, label: input.label, assisted: input.assisted })
            return listMine(userId)
        },

        listDisplayed: async (userIds: string[]) => {
            if (userIds.length === 0) return new Map<string, DisplayedDan[]>()
            const [rows, displays] = await Promise.all([deps.db.listUserDans(userIds), deps.db.listDisplays(userIds)])
            return new Map(
                userIds.map((userId): [string, DisplayedDan[]] => {
                    const rowsOfUser = rows.filter((row) => row.userId === userId)
                    const displaysOfUser = displays.filter((row) => row.userId === userId)
                    return [userId, DAN_SYSTEMS.flatMap((system) => systemViewOf(system, rowsOfUser, displaysOfUser).shown)]
                }),
            )
        },
    }
}

export type DanService = ReturnType<typeof createDanService>
