import { describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { bundledCoursesFileSchema } from '@server/dto/course-catalog'
import type { CourseMetaInput } from '@server/dto/course'
import { isAppError } from '@server/lib/error'
import { ERROR_CODE } from '@server/lib/error-code'
import { bundledCatalog, GENOSIDE_2018_SOURCE_ID } from '@server/service/domain/course-catalog/bundled-courses'
import { createCourseCatalogService } from '@server/service/domain/course-catalog/course-catalog.service'
import type { CatalogDefinition } from '@server/service/domain/course-catalog/course-catalog.type'
import { courseHashOf } from '@server/service/domain/course-catalog/course-hash'
import { tableCatalog } from '@server/service/domain/course-catalog/table-courses'
import { createCourseService } from '@server/service/domain/course/course.service'
import { createMemoryCourseCatalogDb } from '../support/course-catalog-memory-db'

const COURSES_FILE = join(import.meta.dirname, '..', '..', 'data', 'courses', 'genoside-2018', 'courses.json')
const SP_GROUP = 'genoside-2018-sp'
const DP_GROUP = 'genoside-2018-dp'
const SP_NAME_PREFIX = 'GENOSIDE 2018 段位認定 '
const DP_NAME_PREFIX = 'GENOSIDE 2018 DP段位認定 '
const SYNTHETIC_FILE_SHA256 = 'c'.repeat(64)
const RETRIEVED_ON = '2026-10-23'
const RETRIEVED_AT = Date.UTC(2026, 9, 23)
const MD5_LENGTH = 32
const REAL_GROUP_SIZE = 22
const REAL_COURSE_COUNT = REAL_GROUP_SIZE * 2

const md5Of = (seed: string) => createHash('md5').update(seed).digest('hex')

type SyntheticCourse = { name: string; seeds?: string[]; constraint?: string[] }

const syntheticCourse = ({ name, seeds = [name], constraint = ['GRADE_MIRROR', 'gauge_lr2', 'LN'] }: SyntheticCourse) => {
    const charts = seeds.map((seed) => ({ md5: md5Of(seed), sha256: '', title: `Title of ${seed}` }))
    return {
        course_hash: courseHashOf({ name, charts }),
        name,
        charts,
        constraint,
        trophy: [{ name: 'silvermedal', missrate: 5, scorerate: 70 }],
        release: true,
    }
}

const syntheticFile = (groups: { id: string; name: string; keys: number; courses: SyntheticCourse[] }[]) =>
    bundledCoursesFileSchema.parse({
        schema: 1,
        source: {
            maker: 'Maker',
            page_url: 'https://example.test/page',
            file_url: 'https://example.test/file.zip',
            retrieved_on: RETRIEVED_ON,
            file_bytes: 10,
            file_sha256: SYNTHETIC_FILE_SHA256,
            entry_name: 'test.lr2crs',
            entry_sha256: 'd'.repeat(64),
            entry_hash_prefix: '00',
            entry_type: 2,
            constraint_sources: [],
            title_source: 'https://example.test/titles',
        },
        groups: groups.map((group) => ({ ...group, courses: group.courses.map(syntheticCourse) })),
    })

const SP_COURSES: SyntheticCourse[] = [{ name: `${SP_NAME_PREFIX}初段` }, { name: `${SP_NAME_PREFIX}二段` }, { name: `${SP_NAME_PREFIX}三段` }]
const DP_COURSES: SyntheticCourse[] = [{ name: `${DP_NAME_PREFIX}初段` }, { name: 'A course outside the dan names' }]

const genoside = (spCourses = SP_COURSES, dpCourses = DP_COURSES) =>
    bundledCatalog(
        syntheticFile([
            { id: SP_GROUP, name: 'GENOSIDE 2018 段位認定', keys: 7, courses: spCourses },
            { id: DP_GROUP, name: 'GENOSIDE 2018 DP段位認定', keys: 14, courses: dpCourses },
        ]),
    )

const stCourse = (level: number) => ({
    name: `Stella Skill Simulator 4th st${level}`,
    md5: [md5Of(`st${level}a`), md5Of(`st${level}b`)],
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
})

const stella = (levels: number[]): CatalogDefinition =>
    tableCatalog({
        table: {
            id: 'stella-st',
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
        headerCourse: [levels.map(stCourse)],
        entries: [],
        headerSha256: 'a'.repeat(64),
        dataSha256: 'b'.repeat(64),
        dataUrl: 'https://example.test/st/score.json',
        fetchedAt: RETRIEVED_AT,
    }).definition

const createService = () => {
    const memory = createMemoryCourseCatalogDb()
    return { ...memory, service: createCourseCatalogService({ db: memory.db }) }
}

const readFileDefinition = () => bundledCatalog(bundledCoursesFileSchema.parse(JSON.parse(readFileSync(COURSES_FILE, 'utf8'))))

describe('bundledCatalog', () => {
    test('파일의 course_hash 가 계산과 다르면 등록을 멈춘다', () => {
        const file = syntheticFile([{ id: SP_GROUP, name: 'SP', keys: 7, courses: SP_COURSES }])
        file.groups[0].courses[1].course_hash = 'f'.repeat(64)
        try {
            bundledCatalog(file)
            expect.unreachable('a file written under another id rule must be refused')
        } catch (error) {
            expect(isAppError(error) && error.code).toBe(ERROR_CODE.VALIDATION_ERROR)
            expect(isAppError(error) && error.details).toEqual({ field: `${SP_GROUP}[1].course_hash`, reason: 'not the hash of the name and charts' })
        }
    })

    test('같은 코스가 두 번 적힌 파일은 등록을 멈춘다', () => {
        const file = syntheticFile([{ id: SP_GROUP, name: 'SP', keys: 7, courses: [SP_COURSES[0], SP_COURSES[0]] }])
        expect(() => bundledCatalog(file)).toThrow()
    })

    test('달력에 없는 받은 날은 등록을 멈춘다', () => {
        const file = syntheticFile([{ id: SP_GROUP, name: 'SP', keys: 7, courses: SP_COURSES }])
        file.source.retrieved_on = '2026-13-45'
        expect(() => bundledCatalog(file)).toThrow()
    })

    test('출처는 파일의 머리에서, 묶음 순서는 파일 순서에서 만든다', () => {
        const { source, groups } = genoside()
        expect(source).toMatchObject({
            id: GENOSIDE_2018_SOURCE_ID,
            kind: 'bundled',
            maker: 'Maker',
            pageUrl: 'https://example.test/page',
            fileUrl: 'https://example.test/file.zip',
            retrievedAt: RETRIEVED_AT,
            fileSha256: SYNTHETIC_FILE_SHA256,
        })
        expect(source.detail).toMatchObject({ entry_name: 'test.lr2crs', entry_type: 2, title_source: 'https://example.test/titles', file_bytes: 10 })
        expect(source.detail).not.toHaveProperty('maker')
        expect(groups.map((group) => [group.id, group.position, group.keys, group.sourceId])).toEqual([
            [SP_GROUP, 0, 7, GENOSIDE_2018_SOURCE_ID],
            [DP_GROUP, 1, 14, GENOSIDE_2018_SOURCE_ID],
        ])
    })

    test('제약은 정규화하고 트로피의 missrate 는 smissrate 로 옮긴다', () => {
        const [first] = genoside().groups[0].courses
        expect(first.constraint).toEqual(['grade_mirror', 'gauge_lr2', 'ln'])
        expect(first.lntype).toBe(0)
        expect(first.trophy).toEqual([{ name: 'silvermedal', scorerate: 70, smissrate: 5 }])
        expect(first.charts).toEqual([{ md5: md5Of(`${SP_NAME_PREFIX}初段`), sha256: '', title: `Title of ${SP_NAME_PREFIX}初段` }])
    })

    test('단위는 묶음 안의 순서와 이름에서 정해진다', () => {
        const { groups } = genoside()
        expect(groups[0].courses.map((course) => course.dan)).toEqual([
            { system: 'genoside2018_sp', label: '初段', rank: 1 },
            { system: 'genoside2018_sp', label: '二段', rank: 2 },
            { system: 'genoside2018_sp', label: '三段', rank: 3 },
        ])
        expect(groups[1].courses.map((course) => course.dan)).toEqual([{ system: 'genoside2018_dp', label: '初段', rank: 1 }, null])
    })

    test('저장소의 GENOSIDE 2018 파일은 SP 22 와 DP 22 의 단위 코스 44개가 된다', () => {
        const { groups } = readFileDefinition()
        expect(groups.map((group) => [group.id, group.courses.length])).toEqual([
            [SP_GROUP, REAL_GROUP_SIZE],
            [DP_GROUP, REAL_GROUP_SIZE],
        ])
        const courses = groups.flatMap((group) => group.courses)
        expect(
            courses.every((course) => course.dan !== null && course.charts.every((chart) => chart.md5.length === MD5_LENGTH && chart.sha256 === '')),
        ).toBe(true)
        expect(new Set(courses.map((course) => course.courseHash)).size).toBe(REAL_COURSE_COUNT)
        expect(groups[0].courses[REAL_GROUP_SIZE - 1].dan).toEqual({ system: 'genoside2018_sp', label: 'Overjoy', rank: REAL_GROUP_SIZE })
    })
})

describe('createCourseCatalogService', () => {
    test('처음 동기화는 모든 코스를 넣고 단위 코스를 센다', async () => {
        const { service, courses, members } = createService()
        const definition = genoside()
        const plan = await service.applySync(definition)
        expect(plan.groups).toMatchObject([
            { id: SP_GROUP, inserted: 3, changed: 0, unchanged: 0, deactivated: 0, danCourses: 3 },
            { id: DP_GROUP, inserted: 2, changed: 0, unchanged: 0, deactivated: 0, danCourses: 1 },
        ])
        expect(plan.groups[0].newDanHashes).toEqual(definition.groups[0].courses.map((course) => course.courseHash))
        expect(plan.groups[1].newDanHashes).toEqual([definition.groups[1].courses[0].courseHash])
        expect(courses.size).toBe(5)
        expect([...members.values()].every((member) => member.active)).toBe(true)
    })

    test('같은 정의를 다시 동기화하면 바뀌는 것이 없다', async () => {
        const { service } = createService()
        const definition = genoside()
        await service.applySync(definition)
        const again = await service.applySync(definition)
        expect(
            again.groups.map(({ id, inserted, changed, unchanged, deactivated, newDanHashes }) => [
                id,
                inserted,
                changed,
                unchanged,
                deactivated,
                newDanHashes.length,
            ]),
        ).toEqual([
            [SP_GROUP, 0, 0, 3, 0, 0],
            [DP_GROUP, 0, 0, 2, 0, 0],
        ])
    })

    test('계획(planSync)은 쓰지 않고 applySync 와 같은 숫자를 낸다', async () => {
        const { service, counts, courses, members } = createService()
        const definition = genoside()
        const planned = await service.planSync(definition)
        expect(counts.applyGroup).toBe(0)
        expect(courses.size + members.size).toBe(0)
        expect(await service.applySync(definition)).toEqual(planned)
    })

    test('트로피가 바뀐 코스는 바뀐 것으로 세고 갱신한다', async () => {
        const { service } = createService()
        await service.applySync(genoside())
        const edited = genoside()
        edited.groups[0].courses[1].trophy = [{ name: 'goldmedal', scorerate: 90, smissrate: 2 }]
        const plan = await service.planSync(edited)
        expect(plan.groups[0]).toMatchObject({ inserted: 0, changed: 1, unchanged: 2 })
        await service.applySync(edited)
        expect((await service.planSync(edited)).groups[0]).toMatchObject({ changed: 0, unchanged: 3 })
    })

    test('묶음 안의 순서가 바뀐 코스는 바뀐 것으로 센다', async () => {
        const { service } = createService()
        await service.applySync(genoside())
        const [first, second, third] = SP_COURSES
        const reordered = genoside([second, first, third])
        const plan = await service.planSync(reordered)
        expect(plan.groups[0]).toMatchObject({ inserted: 0, changed: 2, unchanged: 1 })
    })

    test('표에서 사라진 코스는 비활성이 되고 단위 코스로 남는다', async () => {
        const { service, courses, members } = createService()
        await service.applySync(stella([0, 1, 2]))
        const plan = await service.applySync(stella([0, 1]))
        expect(plan.groups[0]).toMatchObject({ inserted: 0, changed: 0, unchanged: 2, deactivated: 1 })
        const dropped = stella([2]).groups[0].courses[0]
        expect([...members.values()].find((member) => member.courseHash === dropped.courseHash)?.active).toBe(false)
        expect(courses.get(dropped.courseHash)?.dan).toEqual({ system: 'stella', label: 'st2', rank: 2 })
        expect((await service.planSync(stella([0, 1]))).groups[0].deactivated).toBe(0)
    })

    test('다시 나타난 코스는 활성으로 돌아온다', async () => {
        const { service, members } = createService()
        await service.applySync(stella([0, 1]))
        await service.applySync(stella([0]))
        const plan = await service.applySync(stella([0, 1]))
        expect(plan.groups[0]).toMatchObject({ inserted: 0, changed: 1, unchanged: 1, deactivated: 0 })
        expect([...members.values()].every((member) => member.active)).toBe(true)
    })

    test('코스를 내지 않는 표는 기존 코스를 모두 비활성으로 한다', async () => {
        const { service, members } = createService()
        await service.applySync(stella([0, 1]))
        const plan = await service.applySync(stella([]))
        expect(plan.groups[0]).toMatchObject({ inserted: 0, deactivated: 2 })
        expect([...members.values()].some((member) => member.active)).toBe(false)
    })

    test('새 단위 코스만 소급 대상으로 알린다', async () => {
        const { service } = createService()
        await service.applySync(stella([0]))
        const plan = await service.applySync(stella([0, 1]))
        expect(plan.groups[0].newDanHashes).toEqual([stella([1]).groups[0].courses[0].courseHash])
    })

    test('목록은 묶음 순서대로 출처와 단위를 싣는다', async () => {
        const { service } = createService()
        await service.applySync(stella([0, 1]))
        await service.applySync(genoside())
        const { groups } = await service.listGroups()
        expect(groups.map((group) => [group.id, group.courses.length])).toEqual([
            [SP_GROUP, 3],
            [DP_GROUP, 2],
            ['stella-st', 2],
        ])
        expect(groups[0].source).toMatchObject({ id: GENOSIDE_2018_SOURCE_ID, kind: 'bundled', maker: 'Maker', retrieved_at: RETRIEVED_AT })
        expect(groups[2].source).toMatchObject({ id: 'stella-st', kind: 'table', detail: { data_url: 'https://example.test/st/score.json' } })
        expect(groups[0].courses[0]).toEqual({
            course_hash: genoside().groups[0].courses[0].courseHash,
            name: `${SP_NAME_PREFIX}初段`,
            stage_count: 1,
            active: true,
            dan: { system: 'genoside2018_sp', system_name: 'GENOSIDE 2018 SP', label: '初段', rank: 1 },
        })
        expect(groups[1].courses[1].dan).toBeNull()
    })

    test('한 코스의 상세는 묶음·출처·단위와 차트의 md5 와 제목을 싣는다', async () => {
        const { service } = createService()
        await service.applySync(genoside())
        const target = genoside().groups[0].courses[2]
        const entry = await service.getEntry(target.courseHash)
        expect(entry).toMatchObject({
            course_hash: target.courseHash,
            name: `${SP_NAME_PREFIX}三段`,
            active: true,
            group: { id: SP_GROUP, name: 'GENOSIDE 2018 段位認定', keys: 7 },
            source: { id: GENOSIDE_2018_SOURCE_ID, maker: 'Maker' },
            dan: { system: 'genoside2018_sp', system_name: 'GENOSIDE 2018 SP', label: '三段', rank: 3 },
        })
        expect(entry?.charts).toEqual([{ position: 0, md5: md5Of(`${SP_NAME_PREFIX}三段`), sha256: '', title: `Title of ${SP_NAME_PREFIX}三段` }])
    })

    test('등록되지 않은 코스의 상세는 없다', async () => {
        const { service } = createService()
        expect(await service.getEntry('f'.repeat(64))).toBeNull()
    })
})

describe('관리자 정의 등록(upsertMeta)', () => {
    const META_MD5 = 'a'.repeat(MD5_LENGTH)
    const META_SHA256 = 'b'.repeat(64)

    const meta = (charts: CourseMetaInput['charts']): CourseMetaInput => ({
        course_hash: 'meta-course',
        name: 'Meta course',
        lntype: 0,
        charts,
        constraint: [],
        trophy: [],
        extra: {},
    })

    const registerThrough = async (charts: CourseMetaInput['charts']) => {
        const replaced: { courseHash: string; charts: { position: number; chartSha256: string; md5: string }[] }[] = []
        const service = createCourseService({
            newId: (prefix) => `${prefix}1`,
            db: {
                findCourse: async () => null,
                findCourseCharts: async () => [],
                upsertCourse: async () => undefined,
                replaceCourseCharts: async (courseHash, stages) => {
                    replaced.push({ courseHash, charts: stages })
                },
                findCourseScoreByIdempotency: async () => null,
                insertCourseScore: async () => undefined,
                getCourseBest: async () => null,
                upsertCourseBestIfBetter: async () => false,
                countBetterCourseBests: async () => 0,
                getCourseRanking: async () => [],
                getCourseBestByLoginId: async () => null,
                getRivalUserIds: async () => [],
            },
        })
        await service.upsertMeta(meta(charts))
        return replaced
    }

    test('md5 만 있는 차트도 버리지 않고 md5 와 함께 저장한다', async () => {
        expect(await registerThrough([{ md5: META_MD5, sha256: '' }])).toEqual([
            { courseHash: 'meta-course', charts: [{ position: 0, chartSha256: '', md5: META_MD5 }] },
        ])
    })

    test('sha256 만 있는 차트와 둘 다 있는 차트도 남기고 둘 다 빈 차트만 버린다', async () => {
        const [replaced] = await registerThrough([
            { md5: '', sha256: META_SHA256 },
            { md5: '', sha256: '' },
            { md5: META_MD5, sha256: META_SHA256 },
        ])
        expect(replaced.charts).toEqual([
            { position: 0, chartSha256: META_SHA256, md5: '' },
            { position: 2, chartSha256: META_SHA256, md5: META_MD5 },
        ])
    })
})
