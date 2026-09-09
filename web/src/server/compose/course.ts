import { and, asc, count, desc, eq, inArray, or, sql } from 'drizzle-orm'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { chart, course, courseBest, courseChart, courseScore, rival } from '@server/db/schema'
import { newId } from '@server/lib/token'
import { createCourseService } from '@server/service/domain/course/course.service'
import type { ScoreRankingRow } from '@server/service/domain/score/score.service'

const courseRankingSelection = {
    scoreId: courseScore.id,
    loginId: user.loginId,
    playerName: user.name,
    clear: courseScore.clear,
    exScore: courseScore.exScore,
    maxCombo: courseScore.maxCombo,
    minbp: courseScore.minbp,
    playedAt: courseScore.playedAt,
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
}

type CourseRankingSelectionRow = {
    scoreId: string
    loginId: string
    playerName: string
    clear: number
    exScore: number
    maxCombo: number
    minbp: number
    playedAt: number
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
}

const toCourseRankingRow = (row: CourseRankingSelectionRow): ScoreRankingRow => ({
    scoreId: row.scoreId,
    loginId: row.loginId,
    playerName: row.playerName,
    clear: row.clear,
    exScore: row.exScore,
    maxCombo: row.maxCombo,
    minbp: row.minbp,
    playedAt: row.playedAt,
    lntype: 0,
    option: 0,
    totalNotes: 0,
    judge: {
        pgreat: row.epg + row.lpg,
        great: row.egr + row.lgr,
        good: row.egd + row.lgd,
        bad: row.ebd + row.lbd,
        poor: row.epr + row.lpr,
        miss: row.ems + row.lms,
        fast: 0,
        slow: 0,
        combobreak: 0,
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
        empty_poor: 0,
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
            upsertCourse: async (row) => {
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
            },
            replaceCourseCharts: async (courseHash, charts) =>
                db.transaction(async (tx) => {
                    await tx.delete(courseChart).where(eq(courseChart.courseHash, courseHash))
                    if (charts.length === 0) return
                    await tx.insert(courseChart).values(charts.map((entry) => ({ ...entry, courseHash })))
                }),
            findCourseScoreIdByIdempotency: async ({ userId, courseHash, playedAt }) => {
                const [row] = await db
                    .select({ id: courseScore.id })
                    .from(courseScore)
                    .where(and(eq(courseScore.userId, userId), eq(courseScore.courseHash, courseHash), eq(courseScore.playedAt, playedAt)))
                    .limit(1)
                return row?.id ?? null
            },
            insertCourseScore: async (row) => {
                await db.insert(courseScore).values(row)
            },
            getCourseBest: async ({ courseHash, userId }) => {
                const [row] = await db
                    .select({ clear: courseBest.clear, exScore: courseBest.exScore, minbp: courseBest.minbp })
                    .from(courseBest)
                    .where(and(eq(courseBest.courseHash, courseHash), eq(courseBest.userId, userId)))
                    .limit(1)
                return row ?? null
            },
            upsertCourseBest: async (row) => {
                await db
                    .insert(courseBest)
                    .values(row)
                    .onDuplicateKeyUpdate({
                        set: { courseScoreId: row.courseScoreId, clear: row.clear, exScore: row.exScore, minbp: row.minbp, maxCombo: row.maxCombo },
                    })
            },
            countBetterCourseBests: async ({ courseHash, clear, exScore }) => {
                const [row] = await db
                    .select({ value: count() })
                    .from(courseBest)
                    .where(
                        and(
                            eq(courseBest.courseHash, courseHash),
                            or(sql`${courseBest.clear} > ${clear}`, and(eq(courseBest.clear, clear), sql`${courseBest.exScore} > ${exScore}`)),
                        ),
                    )
                return row?.value ?? 0
            },
            getCourseRanking: async ({ courseHash, limit, offset, userIds }) => {
                const rows = await db
                    .select(courseRankingSelection)
                    .from(courseBest)
                    .innerJoin(courseScore, eq(courseScore.id, courseBest.courseScoreId))
                    .innerJoin(user, eq(user.id, courseBest.userId))
                    .where(and(eq(courseBest.courseHash, courseHash), userIds ? inArray(courseBest.userId, userIds) : undefined))
                    .orderBy(desc(courseBest.clear), desc(courseBest.exScore), asc(courseBest.minbp))
                    .limit(limit)
                    .offset(offset)
                return rows.map(toCourseRankingRow)
            },
            getCourseBestByLoginId: async ({ courseHash, loginId }) => {
                const [row] = await db
                    .select(courseRankingSelection)
                    .from(courseBest)
                    .innerJoin(courseScore, eq(courseScore.id, courseBest.courseScoreId))
                    .innerJoin(user, eq(user.id, courseBest.userId))
                    .where(and(eq(courseBest.courseHash, courseHash), eq(user.loginId, loginId)))
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
