import { afterAll, beforeEach, describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { count, eq, inArray } from 'drizzle-orm'
import { compose } from '@server/compose'
import { composeCourseCatalog } from '@server/compose/course-catalog'
import { getDb } from '@server/db'
import { course, courseChart, courseGroup, courseGroupMember, courseSource, danCourse } from '@server/db/schema'
import { bundledCoursesFileSchema } from '@server/dto/course-catalog'
import { bundledCatalog } from '@server/service/domain/course-catalog/bundled-courses'
import type { CatalogDefinition } from '@server/service/domain/course-catalog/course-catalog.type'
import { tableCatalog } from '@server/service/domain/course-catalog/table-courses'

const hasDatabase = Boolean(process.env.RBMS_DB_E2E)
if (!hasDatabase) {
    console.log('[skip] 코스 카탈로그 DB 통합 테스트 건너뜀 — 마이그레이션을 마친 임시 MySQL 을 DATABASE_URL 로 주고 RBMS_DB_E2E=1 로 실행')
}

const COURSES_FILE = join(import.meta.dirname, '..', '..', 'data', 'courses', 'genoside-2018', 'courses.json')
const SP_GROUP = 'genoside-2018-sp'
const DP_GROUP = 'genoside-2018-dp'
const SOURCE_ID = 'genoside-2018'
const STELLA_GROUP = 'stella-st'
const GROUP_SIZE = 22
const COURSE_COUNT = GROUP_SIZE * 2
const STAGE_COUNT = 4
const MD5_LENGTH = 32
const META_PREFIX = 'cc_meta'
const META_COURSE_HASH = `${META_PREFIX}_course`
const MD5_ONLY_CHART = 'a'.repeat(MD5_LENGTH)
const SHA256_ONLY_CHART = 'b'.repeat(64)
const STELLA_LEVELS = [0, 1, 2]
const KEPT_STELLA_LEVELS = [0, 1]

const md5Of = (seed: string) => createHash('md5').update(seed).digest('hex')

const genoside = () => bundledCatalog(bundledCoursesFileSchema.parse(JSON.parse(readFileSync(COURSES_FILE, 'utf8'))))

const stella = (levels: number[]) =>
    tableCatalog({
        table: {
            id: STELLA_GROUP,
            expectedName: 'Stella',
            symbol: 'st',
            sourceName: 'Stella',
            maker: 'Stella',
            pageUrl: 'https://example.test/st',
            headerUrl: 'https://example.test/st/header.json',
            allowedHost: 'example.test',
            keys: 7,
            groupPosition: 2,
        },
        headerCourse: [
            levels.map((level) => ({ name: `Stella Skill Simulator 4th st${level}`, md5: [md5Of(`cc-st${level}a`), md5Of(`cc-st${level}b`)] })),
        ],
        entries: [{ md5: md5Of('cc-st0a'), title: 'First stage of st0' }],
        headerSha256: 'a'.repeat(64),
        dataSha256: 'b'.repeat(64),
        dataUrl: 'https://example.test/st/score.json',
        fetchedAt: Date.UTC(2026, 9, 23, 15),
    }).definition

const hashesOf = (definition: CatalogDefinition) => definition.groups.flatMap((group) => group.courses.map((entry) => entry.courseHash))

const hashesToClean = () => [...hashesOf(genoside()), ...hashesOf(stella(STELLA_LEVELS)), META_COURSE_HASH]

const GROUP_IDS = [SP_GROUP, DP_GROUP, STELLA_GROUP]
const SOURCE_IDS = [SOURCE_ID, STELLA_GROUP]

const cleanCatalogRows = async () => {
    const db = getDb()
    const hashes = hashesToClean()
    await db.delete(danCourse).where(inArray(danCourse.courseHash, hashes))
    await db.delete(courseGroupMember).where(inArray(courseGroupMember.groupId, GROUP_IDS))
    await db.delete(courseGroup).where(inArray(courseGroup.id, GROUP_IDS))
    await db.delete(courseSource).where(inArray(courseSource.id, SOURCE_IDS))
    await db.delete(courseChart).where(inArray(courseChart.courseHash, hashes))
    await db.delete(course).where(inArray(course.courseHash, hashes))
}

const rowCounts = async (hashes: string[], groupIds: string[]) => {
    const db = getDb()
    const [[courses], [charts], [members], [dans], [groups]] = await Promise.all([
        db.select({ rows: count() }).from(course).where(inArray(course.courseHash, hashes)),
        db.select({ rows: count() }).from(courseChart).where(inArray(courseChart.courseHash, hashes)),
        db.select({ rows: count() }).from(courseGroupMember).where(inArray(courseGroupMember.groupId, groupIds)),
        db.select({ rows: count() }).from(danCourse).where(inArray(danCourse.courseHash, hashes)),
        db.select({ rows: count() }).from(courseGroup).where(inArray(courseGroup.id, groupIds)),
    ])
    return { courses: courses.rows, charts: charts.rows, members: members.rows, dans: dans.rows, groups: groups.rows }
}

describe.skipIf(!hasDatabase)('코스 카탈로그 등록 (임시 MySQL)', () => {
    const catalog = () => composeCourseCatalog(getDb()).courseCatalogService

    beforeEach(cleanCatalogRows)
    afterAll(cleanCatalogRows)

    test('apply 를 두 번 돌려도 같은 행 수다', async () => {
        const definition = genoside()
        const hashes = hashesOf(definition)
        const first = await catalog().applySync(definition)
        expect(first.groups.map((group) => [group.id, group.inserted, group.danCourses])).toEqual([
            [SP_GROUP, GROUP_SIZE, GROUP_SIZE],
            [DP_GROUP, GROUP_SIZE, GROUP_SIZE],
        ])
        const afterFirst = await rowCounts(hashes, [SP_GROUP, DP_GROUP])
        expect(afterFirst).toEqual({
            courses: COURSE_COUNT,
            charts: COURSE_COUNT * STAGE_COUNT,
            members: COURSE_COUNT,
            dans: COURSE_COUNT,
            groups: 2,
        })

        const second = await catalog().applySync(definition)
        expect(second.groups.map((group) => [group.id, group.inserted, group.changed, group.unchanged, group.deactivated])).toEqual([
            [SP_GROUP, 0, 0, GROUP_SIZE, 0],
            [DP_GROUP, 0, 0, GROUP_SIZE, 0],
        ])
        expect(await rowCounts(hashes, [SP_GROUP, DP_GROUP])).toEqual(afterFirst)
    })

    test('GENOSIDE 코스 상세의 차트는 md5 와 제목을 돌려준다', async () => {
        await catalog().applySync(genoside())
        const [target] = genoside().groups[0].courses
        const entry = await catalog().getEntry(target.courseHash)
        expect(entry).toMatchObject({
            name: target.name,
            active: true,
            group: { id: SP_GROUP, keys: 7 },
            source: { id: SOURCE_ID, kind: 'bundled' },
            dan: { system: 'genoside2018_sp', label: '初段', rank: 1 },
        })
        expect(entry?.charts).toEqual(target.charts.map((chart, position) => ({ position, md5: chart.md5, sha256: '', title: chart.title })))
        expect(entry?.charts.every((chart) => chart.md5.length === MD5_LENGTH && chart.title.length > 0)).toBe(true)
    })

    test('코스 정의를 읽는 쪽은 md5 만 있는 차트를 sha256 이 빈 채로 받는다', async () => {
        await catalog().applySync(genoside())
        const [target] = genoside().groups[0].courses
        const meta = await compose().courseService.getMeta(target.courseHash)
        expect(meta?.charts).toEqual(target.charts.map((chart) => ({ md5: chart.md5, sha256: '' })))
        expect(meta?.constraint).toEqual(['grade_mirror', 'gauge_lr2', 'ln'])
    })

    test('목록은 묶음마다 코스 22개와 단계 수를 싣는다', async () => {
        await catalog().applySync(genoside())
        const { groups } = await catalog().listGroups()
        const listed = groups.filter((group) => group.id === SP_GROUP || group.id === DP_GROUP)
        expect(listed.map((group) => [group.id, group.courses.length])).toEqual([
            [SP_GROUP, GROUP_SIZE],
            [DP_GROUP, GROUP_SIZE],
        ])
        expect(listed[0].courses.every((entry) => entry.stage_count === STAGE_COUNT && entry.active && entry.dan !== null)).toBe(true)
        expect(listed[0].source).toMatchObject({ id: SOURCE_ID, retrieved_at: Date.UTC(2026, 9, 23) })
    })

    test('표에서 사라진 코스는 비활성이 되고 dan_course 는 남는다', async () => {
        const hashes = hashesOf(stella(STELLA_LEVELS))
        await catalog().applySync(stella(STELLA_LEVELS))
        const plan = await catalog().applySync(stella(KEPT_STELLA_LEVELS))
        expect(plan.groups[0]).toMatchObject({ inserted: 0, changed: 0, unchanged: KEPT_STELLA_LEVELS.length, deactivated: 1 })
        const [dropped] = hashes.slice(-1)
        const [member] = await getDb().select().from(courseGroupMember).where(eq(courseGroupMember.courseHash, dropped))
        const [dan] = await getDb().select().from(danCourse).where(eq(danCourse.courseHash, dropped))
        expect(member.active).toBe(false)
        expect(dan).toMatchObject({ system: 'stella', label: 'st2', rank: 2 })
        expect((await catalog().getEntry(dropped))?.active).toBe(false)
        expect((await catalog().planSync(stella(KEPT_STELLA_LEVELS))).groups[0]).toMatchObject({ changed: 0, deactivated: 0 })
    })

    test('표 코스의 차트 제목은 표 항목의 제목으로 채워진다', async () => {
        await catalog().applySync(stella(KEPT_STELLA_LEVELS))
        const [first] = hashesOf(stella(KEPT_STELLA_LEVELS))
        const entry = await catalog().getEntry(first)
        expect(entry?.charts.map((chart) => chart.title)).toEqual(['First stage of st0', ''])
        expect(entry?.source).toMatchObject({ id: STELLA_GROUP, kind: 'table', detail: { data_url: 'https://example.test/st/score.json' } })
    })

    test('관리자 정의 등록은 md5 만 있는 차트를 버리지 않는다', async () => {
        const { courseService } = compose()
        const stored = await courseService.upsertMeta({
            course_hash: META_COURSE_HASH,
            name: 'Meta course',
            lntype: 0,
            charts: [
                { md5: MD5_ONLY_CHART, sha256: '' },
                { md5: '', sha256: SHA256_ONLY_CHART },
            ],
            constraint: [],
            trophy: [],
            extra: {},
        })
        expect(stored?.charts).toEqual([
            { md5: MD5_ONLY_CHART, sha256: '' },
            { md5: '', sha256: SHA256_ONLY_CHART },
        ])
        const rows = await getDb().select().from(courseChart).where(eq(courseChart.courseHash, META_COURSE_HASH)).orderBy(courseChart.position)
        expect(rows.map((row) => [row.md5, row.chartSha256])).toEqual([
            [MD5_ONLY_CHART, ''],
            ['', SHA256_ONLY_CHART],
        ])
    })
})
