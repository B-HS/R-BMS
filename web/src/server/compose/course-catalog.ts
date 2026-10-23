import { and, asc, count, eq, inArray } from 'drizzle-orm'
import { z } from 'zod'
import type { Database } from '@server/db'
import { chart, course, courseChart, courseGroup, courseGroupMember, courseSource, danCourse } from '@server/db/schema'
import { COURSE_SOURCE_KINDS, danSystemIdSchema } from '@server/dto/course-catalog'
import { withLockRetry } from '@server/lib/with-lock-retry'
import type { CatalogDan, CatalogSource } from '@server/service/domain/course-catalog/course-catalog.type'
import {
    createCourseCatalogService,
    type CatalogGroupRow,
    type CatalogMemberRow,
    type StoredCatalogCourse,
} from '@server/service/domain/course-catalog/course-catalog.service'

const detailSchema = z.record(z.string(), z.unknown())

const toDan = (row: { system: string | null; label: string | null; rank: number | null }): CatalogDan | null => {
    const system = danSystemIdSchema.safeParse(row.system)
    return system.success && row.label !== null && row.rank !== null ? { system: system.data, label: row.label, rank: row.rank } : null
}

const toSource = (row: typeof courseSource.$inferSelect): CatalogSource | null => {
    const kind = COURSE_SOURCE_KINDS.find((candidate) => candidate === row.kind)
    return kind
        ? {
              id: row.id,
              kind,
              name: row.name,
              maker: row.maker,
              pageUrl: row.pageUrl,
              fileUrl: row.fileUrl,
              retrievedAt: row.retrievedAt,
              fileSha256: row.fileSha256,
              detail: detailSchema.catch({}).parse(row.detail),
          }
        : null
}

const toGroupRow = (group: typeof courseGroup.$inferSelect, source: typeof courseSource.$inferSelect): CatalogGroupRow[] => {
    const catalogSource = toSource(source)
    return catalogSource
        ? [{ id: group.id, sourceId: group.sourceId, name: group.name, keys: group.keys, position: group.position, source: catalogSource }]
        : []
}

export const composeCourseCatalog = (db: Database) => {
    const courseCatalogService = createCourseCatalogService({
        db: {
            findGroupMembers: async (groupId) =>
                db
                    .select({ courseHash: courseGroupMember.courseHash, position: courseGroupMember.position, active: courseGroupMember.active })
                    .from(courseGroupMember)
                    .where(eq(courseGroupMember.groupId, groupId)),

            findCourses: async (courseHashes): Promise<StoredCatalogCourse[]> => {
                if (courseHashes.length === 0) return []
                const [courses, charts, dans] = await Promise.all([
                    db.select().from(course).where(inArray(course.courseHash, courseHashes)),
                    db.select().from(courseChart).where(inArray(courseChart.courseHash, courseHashes)).orderBy(asc(courseChart.position)),
                    db.select().from(danCourse).where(inArray(danCourse.courseHash, courseHashes)),
                ])
                return courses.map((row) => ({
                    courseHash: row.courseHash,
                    name: row.name,
                    lntype: row.lntype,
                    constraint: row.constraint,
                    trophy: row.trophy,
                    charts: charts
                        .filter((entry) => entry.courseHash === row.courseHash)
                        .map((entry) => ({ md5: entry.md5, sha256: entry.chartSha256, title: entry.title })),
                    dan: toDan(dans.find((entry) => entry.courseHash === row.courseHash) ?? { system: null, label: null, rank: null }),
                }))
            },

            applyGroup: withLockRetry(async ({ source, group, courses, deactivate }) =>
                db.transaction(async (tx) => {
                    const sourceValues = {
                        kind: source.kind,
                        name: source.name,
                        maker: source.maker,
                        pageUrl: source.pageUrl,
                        fileUrl: source.fileUrl,
                        retrievedAt: source.retrievedAt,
                        fileSha256: source.fileSha256,
                        detail: source.detail,
                        updatedAt: new Date(),
                    }
                    await tx
                        .insert(courseSource)
                        .values({ id: source.id, ...sourceValues })
                        .onDuplicateKeyUpdate({ set: sourceValues })
                    const groupValues = { sourceId: group.sourceId, name: group.name, keys: group.keys, position: group.position }
                    await tx
                        .insert(courseGroup)
                        .values({ id: group.id, ...groupValues })
                        .onDuplicateKeyUpdate({ set: groupValues })
                    for (const [position, entry] of courses.entries()) {
                        const courseValues = {
                            name: entry.name,
                            lntype: entry.lntype,
                            constraint: entry.constraint,
                            trophy: entry.trophy,
                            extra: { source_id: source.id },
                        }
                        await tx
                            .insert(course)
                            .values({ courseHash: entry.courseHash, ...courseValues })
                            .onDuplicateKeyUpdate({ set: courseValues })
                        await tx.delete(courseChart).where(eq(courseChart.courseHash, entry.courseHash))
                        if (entry.charts.length > 0)
                            await tx.insert(courseChart).values(
                                entry.charts.map((stage, index) => ({
                                    courseHash: entry.courseHash,
                                    position: index,
                                    chartSha256: stage.sha256,
                                    md5: stage.md5,
                                    title: stage.title,
                                })),
                            )
                        const memberValues = { position, active: true }
                        await tx
                            .insert(courseGroupMember)
                            .values({ groupId: group.id, courseHash: entry.courseHash, ...memberValues })
                            .onDuplicateKeyUpdate({ set: memberValues })
                        if (entry.dan) {
                            const danValues = { system: entry.dan.system, label: entry.dan.label, rank: entry.dan.rank }
                            await tx
                                .insert(danCourse)
                                .values({ courseHash: entry.courseHash, ...danValues })
                                .onDuplicateKeyUpdate({ set: danValues })
                        } else await tx.delete(danCourse).where(eq(danCourse.courseHash, entry.courseHash))
                    }
                    if (deactivate.length > 0)
                        await tx
                            .update(courseGroupMember)
                            .set({ active: false })
                            .where(and(eq(courseGroupMember.groupId, group.id), inArray(courseGroupMember.courseHash, deactivate)))
                }),
            ),

            listCatalog: async () => {
                const groupRows = await db
                    .select({ group: courseGroup, source: courseSource })
                    .from(courseGroup)
                    .innerJoin(courseSource, eq(courseSource.id, courseGroup.sourceId))
                const memberRows = await db
                    .select({
                        groupId: courseGroupMember.groupId,
                        courseHash: courseGroupMember.courseHash,
                        position: courseGroupMember.position,
                        active: courseGroupMember.active,
                        name: course.name,
                        system: danCourse.system,
                        label: danCourse.label,
                        rank: danCourse.rank,
                    })
                    .from(courseGroupMember)
                    .innerJoin(course, eq(course.courseHash, courseGroupMember.courseHash))
                    .leftJoin(danCourse, eq(danCourse.courseHash, courseGroupMember.courseHash))
                const stageCounts =
                    memberRows.length === 0
                        ? []
                        : await db
                              .select({ courseHash: courseChart.courseHash, stages: count() })
                              .from(courseChart)
                              .where(
                                  inArray(
                                      courseChart.courseHash,
                                      memberRows.map((row) => row.courseHash),
                                  ),
                              )
                              .groupBy(courseChart.courseHash)
                const stagesOf = new Map(stageCounts.map((row) => [row.courseHash, row.stages]))
                const members: CatalogMemberRow[] = memberRows.map((row) => ({
                    groupId: row.groupId,
                    courseHash: row.courseHash,
                    position: row.position,
                    active: row.active,
                    name: row.name,
                    stageCount: stagesOf.get(row.courseHash) ?? 0,
                    dan: toDan(row),
                }))
                return { groups: groupRows.flatMap((row) => toGroupRow(row.group, row.source)), members }
            },

            findEntry: async (courseHash) => {
                const [membership] = await db
                    .select({ group: courseGroup, source: courseSource, active: courseGroupMember.active, name: course.name })
                    .from(courseGroupMember)
                    .innerJoin(course, eq(course.courseHash, courseGroupMember.courseHash))
                    .innerJoin(courseGroup, eq(courseGroup.id, courseGroupMember.groupId))
                    .innerJoin(courseSource, eq(courseSource.id, courseGroup.sourceId))
                    .where(eq(courseGroupMember.courseHash, courseHash))
                    .orderBy(asc(courseGroup.position), asc(courseGroup.id))
                    .limit(1)
                if (!membership) return null
                const [group] = toGroupRow(membership.group, membership.source)
                if (!group) return null
                const [charts, [dan]] = await Promise.all([
                    db
                        .select({ md5: courseChart.md5, chartMd5: chart.md5, sha256: courseChart.chartSha256, title: courseChart.title })
                        .from(courseChart)
                        .leftJoin(chart, eq(chart.sha256, courseChart.chartSha256))
                        .where(eq(courseChart.courseHash, courseHash))
                        .orderBy(asc(courseChart.position)),
                    db.select().from(danCourse).where(eq(danCourse.courseHash, courseHash)).limit(1),
                ])
                return {
                    group,
                    active: membership.active,
                    course: { courseHash, name: membership.name },
                    charts: charts.map((row) => ({ md5: row.md5 || row.chartMd5 || '', sha256: row.sha256, title: row.title })),
                    dan: dan ? toDan(dan) : null,
                }
            },
        },
    })

    return { courseCatalogService }
}
