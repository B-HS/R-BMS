import { and, asc, eq, gt, inArray, isNotNull, or } from 'drizzle-orm'
import { z } from 'zod'
import type { Database } from '@server/db'
import { user } from '@server/db/auth-schema'
import { course, courseChart, courseScore, danCourse, userDan, userDanDisplay } from '@server/db/schema'
import { danSystemIdSchema } from '@server/dto/course-catalog'
import { withLockRetry } from '@server/lib/with-lock-retry'
import { storedFlagsOf } from '@server/service/domain/course/course.service'
import { canonicalConstraints } from '@server/service/domain/course-catalog/constraint-tokens'
import { createDanService, type AcquiredDanRow, type DanCourseRecord, type DanDisplayRow } from '@server/service/domain/dan/dan.service'

const constraintSchema = z.array(z.string()).catch([])

/** Keeps the rows whose system this server knows: a row of another system (written by a newer server) is not shown or judged. */
const withKnownSystem = <T extends { system: string }>(rows: T[]) =>
    rows.flatMap((row) => {
        const system = danSystemIdSchema.safeParse(row.system)
        return system.success ? [{ ...row, system: system.data }] : []
    })

export const composeDan = (db: Database) => {
    const danService = createDanService({
        db: {
            findDanCourses: async (courseHashes): Promise<DanCourseRecord[]> => {
                if (courseHashes.length === 0) return []
                const [dans, courses, charts] = await Promise.all([
                    db.select().from(danCourse).where(inArray(danCourse.courseHash, courseHashes)),
                    db
                        .select({ courseHash: course.courseHash, constraint: course.constraint })
                        .from(course)
                        .where(inArray(course.courseHash, courseHashes)),
                    db
                        .select({ courseHash: courseChart.courseHash, md5: courseChart.md5, sha256: courseChart.chartSha256 })
                        .from(courseChart)
                        .where(inArray(courseChart.courseHash, courseHashes))
                        .orderBy(asc(courseChart.position)),
                ])
                return withKnownSystem(dans).map((dan) => ({
                    courseHash: dan.courseHash,
                    dan: { system: dan.system, label: dan.label, rank: dan.rank },
                    constraint: canonicalConstraints(
                        constraintSchema.parse(courses.find((entry) => entry.courseHash === dan.courseHash)?.constraint),
                    ),
                    charts: charts.filter((entry) => entry.courseHash === dan.courseHash).map(({ md5, sha256 }) => ({ md5, sha256 })),
                }))
            },

            listDanCourseHashes: async () => (await db.select({ courseHash: danCourse.courseHash }).from(danCourse)).map((row) => row.courseHash),

            listCourseScoresWithStages: async ({ courseHashes, after, limit }) => {
                const rows = await db
                    .select({
                        id: courseScore.id,
                        userId: user.id,
                        courseHash: courseScore.courseHash,
                        clear: courseScore.clear,
                        ranked: courseScore.ranked,
                        scratchAuto: courseScore.scratchAuto,
                        gauge: courseScore.gauge,
                        courseGauge: courseScore.courseGauge,
                        gaugeValue: courseScore.gaugeValue,
                        stages: courseScore.stages,
                        playedAt: courseScore.playedAt,
                        extra: courseScore.extra,
                    })
                    .from(courseScore)
                    .innerJoin(user, eq(user.id, courseScore.userId))
                    .where(
                        and(
                            inArray(courseScore.courseHash, courseHashes),
                            isNotNull(courseScore.stages),
                            after
                                ? or(
                                      gt(courseScore.playedAt, after.playedAt),
                                      and(eq(courseScore.playedAt, after.playedAt), gt(courseScore.id, after.id)),
                                  )
                                : undefined,
                        ),
                    )
                    .orderBy(asc(courseScore.playedAt), asc(courseScore.id))
                    .limit(limit)
                return rows.flatMap(({ extra, stages, ...run }) => (stages === null ? [] : [{ ...run, stages, flags: storedFlagsOf(extra) }]))
            },

            insertUserDanIgnore: withLockRetry(async (row) => {
                const [result] = await db.insert(userDan).ignore().values(row)
                return result.affectedRows > 0
            }),

            hasUserDan: async ({ userId, system, label }) => {
                const [row] = await db
                    .select({ userId: userDan.userId })
                    .from(userDan)
                    .where(and(eq(userDan.userId, userId), eq(userDan.system, system), eq(userDan.label, label)))
                    .limit(1)
                return row !== undefined
            },

            listUserDans: async (userIds): Promise<AcquiredDanRow[]> => {
                if (userIds.length === 0) return []
                const rows = await db
                    .select({
                        userId: userDan.userId,
                        system: userDan.system,
                        label: userDan.label,
                        rank: userDan.rank,
                        courseHash: userDan.courseHash,
                        courseScoreId: userDan.courseScoreId,
                        courseGauge: userDan.courseGauge,
                        gauge: userDan.gauge,
                        acquiredAt: userDan.acquiredAt,
                        courseName: course.name,
                    })
                    .from(userDan)
                    .leftJoin(course, eq(course.courseHash, userDan.courseHash))
                    .where(inArray(userDan.userId, userIds))
                    .orderBy(asc(userDan.rank))
                return withKnownSystem(rows).map((row) => ({ ...row, courseName: row.courseName ?? '' }))
            },

            listDisplays: async (userIds): Promise<DanDisplayRow[]> => {
                if (userIds.length === 0) return []
                const rows = await db
                    .select({ userId: userDanDisplay.userId, system: userDanDisplay.system, label: userDanDisplay.label })
                    .from(userDanDisplay)
                    .where(inArray(userDanDisplay.userId, userIds))
                return withKnownSystem(rows)
            },

            upsertDisplay: withLockRetry(async (row) => {
                await db
                    .insert(userDanDisplay)
                    .values(row)
                    .onDuplicateKeyUpdate({ set: { label: row.label, updatedAt: new Date() } })
            }),

            deleteDisplay: withLockRetry(async ({ userId, system }) => {
                await db.delete(userDanDisplay).where(and(eq(userDanDisplay.userId, userId), eq(userDanDisplay.system, system)))
            }),
        },
    })

    return { danService }
}
