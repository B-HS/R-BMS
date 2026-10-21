import { and, asc, count, eq, inArray } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, course, courseBest, courseChart, courseScore, rival } from '@server/db/schema'
import { NO_ASSIST_GRADE, type AssistGrade } from '@server/dto/common'
import { newId } from '@server/lib/token'
import { withLockRetry } from '@server/lib/with-lock-retry'
import { createCourseService } from '@server/service/domain/course/course.service'
import { effectiveJudgeCounts } from '@server/service/domain/score/best-policy'
import type { ScoreRankedRow, ScoreRankingRow } from '@server/service/domain/score/score.service'
import { courseBestScope, isBeatenBy, isRankedAheadOf, listOrder, rankInList } from '@server/compose/best-scope'

const courseRankingSelection = {
    scoreId: courseScore.id,
    loginId: user.loginId,
    playerName: user.name,
    clear: courseScore.clear,
    assistGrade: courseBest.assistGrade,
    exScore: courseScore.exScore,
    maxCombo: courseScore.maxCombo,
    minbp: courseScore.minbp,
    playedAt: courseScore.playedAt,
    scratchAuto: courseScore.scratchAuto,
    epg: courseScore.epg,
    lpg: courseScore.lpg,
    egr: courseScore.egr,
    lgr: courseScore.lgr,
    egd: courseScore.egd,
    lgd: courseScore.lgd,
    ebd: courseScore.ebd,
    lbd: courseScore.lbd,
    epr: courseScore.epr,
    lpr: courseScore.lpr,
    ems: courseScore.ems,
    lms: courseScore.lms,
    pgreat: courseScore.pgreat,
    great: courseScore.great,
    good: courseScore.good,
    bad: courseScore.bad,
    poor: courseScore.poor,
    miss: courseScore.miss,
    fast: courseScore.fast,
    slow: courseScore.slow,
    combobreak: courseScore.combobreak,
    emptyPoor: courseScore.emptyPoor,
}

type CourseRankingSelectionRow = {
    scoreId: string
    loginId: string
    playerName: string
    clear: number
    assistGrade: AssistGrade
    exScore: number
    maxCombo: number
    minbp: number
    playedAt: number
    scratchAuto: boolean
    epg: number
    lpg: number
    egr: number
    lgr: number
    egd: number
    lgd: number
    ebd: number
    lbd: number
    epr: number
    lpr: number
    ems: number
    lms: number
    pgreat: number
    great: number
    good: number
    bad: number
    poor: number
    miss: number
    fast: number
    slow: number
    combobreak: number
    emptyPoor: number
}

const toCourseRankingRow = (row: CourseRankingSelectionRow): ScoreRankingRow => ({
    scoreId: row.scoreId,
    loginId: row.loginId,
    playerName: row.playerName,
    clear: row.clear,
    assistGrade: row.assistGrade,
    exScore: row.exScore,
    maxCombo: row.maxCombo,
    minbp: row.minbp,
    playedAt: row.playedAt,
    lntype: 0,
    option: 0,
    totalNotes: 0,
    scratchAuto: row.scratchAuto,
    judge: {
        ...effectiveJudgeCounts(row),
        fast: row.fast,
        slow: row.slow,
        combobreak: row.combobreak,
        epg: row.epg,
        lpg: row.lpg,
        egr: row.egr,
        lgr: row.lgr,
        egd: row.egd,
        lgd: row.lgd,
        ebd: row.ebd,
        lbd: row.lbd,
        epr: row.epr,
        lpr: row.lpr,
        ems: row.ems,
        lms: row.lms,
        avgjudge: 0,
        empty_poor: row.emptyPoor,
    },
})

export const composeCourse = (db: Database) => {
    const courseService = createCourseService({
        newId,
        db: {
            findCourse: async (courseHash) => {
                const [row] = await db.select().from(course).where(eq(course.courseHash, courseHash)).limit(1)
                return row
                    ? {
                          courseHash: row.courseHash,
                          name: row.name,
                          lntype: row.lntype,
                          constraint: row.constraint,
                          trophy: row.trophy,
                          extra: row.extra,
                      }
                    : null
            },
            findCourseCharts: async (courseHash) => {
                const rows = await db
                    .select({ position: courseChart.position, chartSha256: courseChart.chartSha256, md5: chart.md5 })
                    .from(courseChart)
                    .leftJoin(chart, eq(chart.sha256, courseChart.chartSha256))
                    .where(eq(courseChart.courseHash, courseHash))
                    .orderBy(asc(courseChart.position))
                return rows
            },
            upsertCourse: withLockRetry(async (row) => {
                const values = {
                    courseHash: row.courseHash,
                    name: row.name,
                    lntype: row.lntype,
                    constraint: row.constraint,
                    trophy: row.trophy,
                    extra: row.extra,
                }
                await db
                    .insert(course)
                    .values(values)
                    .onDuplicateKeyUpdate({
                        set: { name: values.name, lntype: values.lntype, constraint: values.constraint, trophy: values.trophy, extra: values.extra },
                    })
            }),
            replaceCourseCharts: withLockRetry(async (courseHash, charts) =>
                db.transaction(async (tx) => {
                    await tx.delete(courseChart).where(eq(courseChart.courseHash, courseHash))
                    if (charts.length === 0) return
                    await tx.insert(courseChart).values(charts.map((entry) => ({ ...entry, courseHash })))
                }),
            ),
            findCourseScoreByIdempotency: async ({ userId, courseHash, playedAt }) => {
                const [row] = await db
                    .select({
                        id: courseScore.id,
                        clear: courseScore.clear,
                        exScore: courseScore.exScore,
                        minbp: courseScore.minbp,
                        maxCombo: courseScore.maxCombo,
                        ranked: courseScore.ranked,
                        scratchAuto: courseScore.scratchAuto,
                    })
                    .from(courseScore)
                    .where(and(eq(courseScore.userId, userId), eq(courseScore.courseHash, courseHash), eq(courseScore.playedAt, playedAt)))
                    .limit(1)
                return row ? { ...row, assistGrade: NO_ASSIST_GRADE } : null
            },
            insertCourseScore: withLockRetry(async (row) => {
                await db.insert(courseScore).values(row)
            }),
            getCourseBest: async ({ courseHash, userId, scratchAuto }) => {
                const [row] = await db
                    .select({ clear: courseBest.clear, assistGrade: courseBest.assistGrade, exScore: courseBest.exScore, minbp: courseBest.minbp })
                    .from(courseBest)
                    .where(and(eq(courseBest.courseHash, courseHash), eq(courseBest.userId, userId), eq(courseBest.scratchAuto, scratchAuto)))
                    .limit(1)
                return row ?? null
            },
            upsertCourseBestIfBetter: withLockRetry(async (row) => {
                const [inserted] = await db.insert(courseBest).ignore().values(row)
                if (inserted.affectedRows > 0) return true
                const [replaced] = await db
                    .update(courseBest)
                    .set({
                        courseScoreId: row.courseScoreId,
                        clear: row.clear,
                        assistGrade: row.assistGrade,
                        exScore: row.exScore,
                        minbp: row.minbp,
                        maxCombo: row.maxCombo,
                    })
                    .where(
                        and(
                            eq(courseBest.courseHash, row.courseHash),
                            eq(courseBest.userId, row.userId),
                            eq(courseBest.scratchAuto, row.scratchAuto),
                            isBeatenBy(courseBest, row),
                        ),
                    )
                return replaced.affectedRows > 0
            }),
            countBetterCourseBests: async ({ courseHash, clear, assistGrade, exScore, minbp, includeScratchAuto }) => {
                const [row] = await db
                    .select({ value: count() })
                    .from(courseBest)
                    .where(
                        and(
                            eq(courseBest.courseHash, courseHash),
                            courseBestScope(db, includeScratchAuto),
                            isRankedAheadOf(courseBest, { clear, assistGrade, exScore, minbp }),
                        ),
                    )
                return row?.value ?? 0
            },
            getCourseRanking: async ({ courseHash, limit, offset, userIds, includeScratchAuto }): Promise<ScoreRankedRow[]> => {
                const rows = await db
                    .select({ ...courseRankingSelection, rank: rankInList(courseBest) })
                    .from(courseBest)
                    .innerJoin(courseScore, eq(courseScore.id, courseBest.courseScoreId))
                    .innerJoin(user, eq(user.id, courseBest.userId))
                    .where(
                        and(
                            eq(courseBest.courseHash, courseHash),
                            courseBestScope(db, includeScratchAuto),
                            userIds ? inArray(courseBest.userId, userIds) : undefined,
                        ),
                    )
                    .orderBy(...listOrder(courseBest))
                    .limit(limit)
                    .offset(offset)
                return rows.map((row) => ({ ...toCourseRankingRow(row), rank: row.rank }))
            },
            getCourseBestByLoginId: async ({ courseHash, loginId, includeScratchAuto }) => {
                const [row] = await db
                    .select(courseRankingSelection)
                    .from(courseBest)
                    .innerJoin(courseScore, eq(courseScore.id, courseBest.courseScoreId))
                    .innerJoin(user, eq(user.id, courseBest.userId))
                    .where(and(eq(courseBest.courseHash, courseHash), courseBestScope(db, includeScratchAuto), eq(user.loginId, loginId)))
                    .limit(1)
                return row ? toCourseRankingRow(row) : null
            },
            getRivalUserIds: async (loginId) => {
                const [owner] = await db.select({ id: user.id }).from(user).where(eq(user.loginId, loginId)).limit(1)
                if (!owner) return []
                const rows = await db.select({ rivalId: rival.rivalId }).from(rival).where(eq(rival.userId, owner.id))
                return [owner.id, ...rows.map((row) => row.rivalId)]
            },
        },
    })

    return { courseService }
}
