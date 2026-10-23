import { describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { DEFAULT_COURSE_NAME, tableCatalog } from '@server/service/domain/course-catalog/table-courses'
import type { TableCatalogSource } from '@server/service/domain/course-catalog/course-catalog.type'

const STELLA_TABLE: TableCatalogSource = {
    id: 'stella-st',
    expectedName: 'Stella',
    sourceName: 'Stella',
    maker: 'Stella',
    pageUrl: 'https://stellabms.xyz/st/table.html',
    headerUrl: 'https://stellabms.xyz/st/header.json',
    keys: 7,
    groupPosition: 3,
}
const HEADER_SHA256 = 'a'.repeat(64)
const DATA_SHA256 = 'b'.repeat(64)
const DATA_URL = 'https://stellabms.xyz/st/score.json'
const FETCHED_AT = Date.UTC(2026, 9, 23, 15)
const MD5_LENGTH = 32
const SHA256_LENGTH = 64
const COLUMN_TEXT_MAX = 255

const md5Of = (seed: string) => createHash('md5').update(seed).digest('hex')
const sha256Of = (seed: string) => createHash('sha256').update(seed).digest('hex')
const md5s = (...seeds: string[]) => seeds.map(md5Of)

const readTable = (headerCourse: unknown, entries: readonly { md5: string; title: string }[] = [], table: TableCatalogSource = STELLA_TABLE) =>
    tableCatalog({ table, headerCourse, entries, headerSha256: HEADER_SHA256, dataSha256: DATA_SHA256, dataUrl: DATA_URL, fetchedAt: FETCHED_AT })

const coursesOf = (result: ReturnType<typeof readTable>) => result.definition.groups[0].courses

const stCourse = (level: number, charts = md5s(`a${level}`, `b${level}`)) => ({
    name: `Stella Skill Simulator 4th st${level}`,
    md5: charts,
    constraint: ['grade_mirror', 'gauge_lr2', 'ln'],
    trophy: [{ name: 'silvermedal', missrate: 10, scorerate: 60 }],
})

describe('tableCatalog — 표 header 의 course 읽기', () => {
    test('배열의 배열을 펴서 순서대로 읽는다', () => {
        const result = readTable([[stCourse(0), stCourse(1)], [stCourse(2)]])
        expect(coursesOf(result).map((course) => course.name)).toEqual([
            'Stella Skill Simulator 4th st0',
            'Stella Skill Simulator 4th st1',
            'Stella Skill Simulator 4th st2',
        ])
        expect(result.skipped).toBe(0)
    })

    test('평평한 배열과 객체 하나도 읽는다', () => {
        expect(coursesOf(readTable([stCourse(0), stCourse(1)]))).toHaveLength(2)
        expect(coursesOf(readTable(stCourse(5)))).toHaveLength(1)
    })

    test('md5 는 소문자로 바뀐다', () => {
        const upper = md5Of('upper').toUpperCase()
        const [course] = coursesOf(readTable([{ name: 'upper', md5: [upper] }]))
        expect(course.charts[0].md5).toBe(upper.toLowerCase())
    })

    test('md5 문자열 하나도 차트 하나가 된다', () => {
        const only = md5Of('only')
        const [course] = coursesOf(readTable([{ name: 'single', md5: only }]))
        expect(course.charts).toEqual([{ md5: only, sha256: '', title: '' }])
    })

    test('charts 객체 배열을 적힌 그대로 읽는다', () => {
        const [first, second] = md5s('first', 'second')
        const [course] = coursesOf(readTable([{ name: 'objects', charts: [{ md5: first, title: 'First' }, { md5: second }], constraint: [] }]))
        expect(course.charts).toEqual([
            { md5: first, sha256: '', title: 'First' },
            { md5: second, sha256: '', title: '' },
        ])
    })

    test('숫자로 적힌 해시는 글자로 읽혀 해시 형식이 아니면 건너뛴다', () => {
        const result = readTable([{ name: 'numeric', md5: [12345] }])
        expect(coursesOf(result)).toHaveLength(0)
        expect(result.skipped).toBe(1)
    })

    test('차트가 없는 코스는 건너뛰고 센다', () => {
        const result = readTable([stCourse(0), { name: 'empty', md5: [] }, { name: 'missing' }, stCourse(1)])
        expect(coursesOf(result).map((course) => course.name)).toEqual(['Stella Skill Simulator 4th st0', 'Stella Skill Simulator 4th st1'])
        expect(result.skipped).toBe(2)
    })

    test('두 해시가 모두 빈 차트가 있는 코스는 건너뛴다', () => {
        const result = readTable([{ name: 'blank', md5: [md5Of('present'), ''] }])
        expect(coursesOf(result)).toHaveLength(0)
        expect(result.skipped).toBe(1)
    })

    test('저장할 수 없는 해시나 긴 이름의 코스는 건너뛴다', () => {
        const result = readTable([
            { name: 'short hash', md5: ['abc123'] },
            { name: 'not hex', md5: ['z'.repeat(MD5_LENGTH)] },
            { name: 'bad sha256', md5: md5s('ok'), sha256: ['f'.repeat(SHA256_LENGTH - 1)] },
            { name: 'n'.repeat(COLUMN_TEXT_MAX + 1), md5: md5s('long name') },
        ])
        expect(coursesOf(result)).toHaveLength(0)
        expect(result.skipped).toBe(4)
    })

    test('객체가 아닌 항목은 건너뛰고 센다', () => {
        const result = readTable([stCourse(0), 'text', null, 7, stCourse(1)])
        expect(coursesOf(result)).toHaveLength(2)
        expect(result.skipped).toBe(3)
    })

    test('같은 id 가 다시 나오면 뒤의 것은 건너뛴다', () => {
        const result = readTable([stCourse(0), stCourse(0)])
        expect(coursesOf(result)).toHaveLength(1)
        expect(result.skipped).toBe(1)
    })

    test('course 가 없는 header 는 코스를 내지 않는다', () => {
        const result = readTable(undefined)
        expect(coursesOf(result)).toEqual([])
        expect(result.skipped).toBe(0)
    })

    test('이름이 비어 있으면 기본 이름을 쓴다', () => {
        const [course] = coursesOf(readTable([{ md5: md5s('unnamed') }]))
        expect(course.name).toBe(DEFAULT_COURSE_NAME)
    })

    test('모르는 제약은 버리고 같은 그룹은 처음 것만 남는다', () => {
        const [course] = coursesOf(
            readTable([{ name: 'constraints', md5: md5s('c'), constraint: ['GRADE_MIRROR', 'grade', 'gauge_lr2', 'LN', 'cn', 'unknown'] }]),
        )
        expect(course.constraint).toEqual(['grade_mirror', 'gauge_lr2', 'ln'])
    })

    test('constraints 와 trophies 라는 이름도 읽는다', () => {
        const [course] = coursesOf(
            readTable([{ name: 'aliases', md5: md5s('a'), constraints: ['hcn'], trophies: [{ name: 'goldmedal', missrate: 5, scorerate: 80 }] }]),
        )
        expect(course.constraint).toEqual(['hcn'])
        expect(course.trophy).toEqual([{ name: 'goldmedal', scorerate: 80, smissrate: 5 }])
    })

    test('트로피의 missrate 는 smissrate 로 옮겨진다', () => {
        const [course] = coursesOf(readTable([stCourse(0)]))
        expect(course.trophy).toEqual([{ name: 'silvermedal', scorerate: 60, smissrate: 10 }])
    })

    test('도달할 수 없는 트로피는 버린다', () => {
        const [course] = coursesOf(
            readTable([
                {
                    name: 'trophies',
                    md5: md5s('t'),
                    trophy: [
                        { name: '', missrate: 10, scorerate: 50 },
                        { name: 'no miss allowed', missrate: 0, scorerate: 50 },
                        { name: 'above the maximum', missrate: 10, scorerate: 100 },
                        { name: 'kept', missrate: 10, scorerate: 99.5 },
                    ],
                },
            ]),
        )
        expect(course.trophy).toEqual([{ name: 'kept', scorerate: 99.5, smissrate: 10 }])
    })

    test('ln 제약 코스의 lntype 은 0, cn 은 1, hcn 은 2 이고 없으면 0 이다', () => {
        const lntypes = ['ln', 'cn', 'hcn', 'grade'].map(
            (token) => coursesOf(readTable([{ name: token, md5: md5s(token), constraint: [token] }]))[0].lntype,
        )
        expect(lntypes).toEqual([0, 1, 2, 0])
    })

    test('제목은 코스가 준 것, 없으면 표 항목의 md5 로 채우고 그것도 없으면 빈 글자다', () => {
        const [given, listed, unknown] = md5s('given', 'listed', 'unknown')
        const entries = [
            { md5: listed.toUpperCase(), title: 'From the table' },
            { md5: listed, title: 'Second listing is ignored' },
            { md5: given, title: 'Not used' },
        ]
        const [course] = coursesOf(
            readTable([{ name: 'titles', charts: [{ md5: given, title: 'From the course' }, { md5: listed }, { md5: unknown }] }], entries),
        )
        expect(course.charts.map((chart) => chart.title)).toEqual(['From the course', 'From the table', ''])
    })

    test('너무 긴 제목은 열 길이로 자른다', () => {
        const [course] = coursesOf(readTable([{ name: 'long title', charts: [{ md5: md5Of('long'), title: 't'.repeat(COLUMN_TEXT_MAX + 5) }] }]))
        expect(course.charts[0].title).toHaveLength(COLUMN_TEXT_MAX)
    })

    test('정의에 없는 sha256 을 채우지 않는다', () => {
        const chartMd5 = md5Of('known to the table')
        const entries = [{ md5: chartMd5, title: 'Known', sha256: sha256Of('known to the table') }]
        const [course] = coursesOf(readTable([{ name: 'no sha256', md5: [chartMd5] }], entries))
        expect(course.charts[0].sha256).toBe('')
    })

    test('header 가 sha256 도 주면 그 값으로 코스 id 를 만든다', () => {
        const chartMd5 = md5Of('with sha256')
        const chartSha256 = sha256Of('with sha256')
        const [course] = coursesOf(readTable([{ name: 'hashed', md5: [chartMd5], sha256: [chartSha256.toUpperCase()] }]))
        expect(course.charts[0].sha256).toBe(chartSha256)
        expect(course.courseHash).toBe(createHash('sha256').update(`hashed\n${chartSha256}\n`).digest('hex'))
    })

    test('Stella Skill Simulator 4th st0 의 id 는 이름과 차트 md5 로 정해진다', () => {
        const [course] = coursesOf(
            readTable([
                {
                    name: 'Stella Skill Simulator 4th st0',
                    md5: [
                        '349bc491ec40d5595412637d8a4c8d2e',
                        'baee0a1921fc5041b44d7d87c7b5548d',
                        '72b1ce4b2051bd2a396dfa11a2d785ee',
                        '3a1661a3eaafa13f976e1010d5b87ca0',
                    ],
                },
            ]),
        )
        expect(course.courseHash).toBe('afce60f187e4ba4ab0519b54d9a0fcf04184e6b137c4826c4513be2d4f99f158')
    })

    test('Stella 표의 Skill Simulator 코스만 단위를 주고 技能訓練 코스는 주지 않는다', () => {
        const courses = coursesOf(readTable([stCourse(0), stCourse(12), { name: 'STELLA技能訓練 4th st3', md5: md5s('training') }]))
        expect(courses.map((course) => course.dan)).toEqual([
            { system: 'stella', label: 'st0', rank: 0 },
            { system: 'stella', label: 'st12', rank: 12 },
            null,
        ])
    })

    test('단위 코스 묶음이 아닌 표의 코스는 단위를 주지 않는다', () => {
        const courses = coursesOf(readTable([stCourse(0)], [], { ...STELLA_TABLE, id: 'some-other-table' }))
        expect(courses[0].dan).toBeNull()
    })

    test('출처와 묶음에 표의 정보를 담는다', () => {
        const { definition } = readTable([stCourse(0)])
        expect(definition.source).toEqual({
            id: 'stella-st',
            kind: 'table',
            name: 'Stella',
            maker: 'Stella',
            pageUrl: 'https://stellabms.xyz/st/table.html',
            fileUrl: 'https://stellabms.xyz/st/header.json',
            retrievedAt: FETCHED_AT,
            fileSha256: HEADER_SHA256,
            detail: { data_url: DATA_URL, data_sha256: DATA_SHA256 },
        })
        expect(definition.groups).toHaveLength(1)
        expect(definition.groups[0]).toMatchObject({ id: 'stella-st', sourceId: 'stella-st', name: 'Stella', keys: 7, position: 3 })
    })
})
