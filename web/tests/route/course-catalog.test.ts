import { beforeEach, describe, expect, test } from 'bun:test'
import { consumeRateLimit, RATE_LIMIT, resetRateLimits } from '@server/lib/with-rate-limit'
import { createCourseCatalogEntryRoute, createCourseCatalogListRoute } from '@server/route/course-catalog'
import {
    CONSTRAINT_SOURCE_URL,
    GENOSIDE_DP_FIRST,
    GENOSIDE_FIRST,
    GENOSIDE_FIRST_TITLE,
    GENOSIDE_PAGE_URL,
    STELLA_PAGE_URL,
    STELLA_TRAINING,
    createCatalogFixture,
} from '../support/course-catalog-fixture'

const ORIGIN = 'https://bms.hyuns.uk'
const CLIENT_IP = '203.0.113.7'
const UNKNOWN_COURSE = 'a-course-no-catalog-lists'
const FIRST_STAGE = 0
const STAGE_COUNT = 4

const createRoutes = async () => {
    const service = await createCatalogFixture()
    const resolveDeps = () => ({ readCatalog: () => service.listGroups(), readEntry: (courseHash: string) => service.getEntry(courseHash) })
    return { list: createCourseCatalogListRoute(resolveDeps), entry: createCourseCatalogEntryRoute(resolveDeps) }
}

const get = (path: string) => new Request(`${ORIGIN}${path}`, { headers: { 'x-forwarded-for': CLIENT_IP } })

const entryContext = (courseHash: string) => ({ params: Promise.resolve({ courseHash }) })

beforeEach(() => resetRateLimits())

describe('GET /api/fe/courses', () => {
    test('코스 묶음 목록은 출처와 단위를 싣는다', async () => {
        const { list } = await createRoutes()
        const response = await list(get('/api/fe/courses'), undefined)
        expect(response.status).toBe(200)
        const { success, data } = await response.json()
        expect(success).toBe(true)
        expect(data.groups.map((group: { id: string }) => group.id)).toEqual(['genoside-2018-sp', 'genoside-2018-dp', 'stella-st'])
        const [genoside, , stella] = data.groups
        expect(genoside.source).toMatchObject({ kind: 'bundled', maker: 'GENOCIDE', page_url: GENOSIDE_PAGE_URL })
        expect(genoside.source.detail.constraint_sources[0].url).toBe(CONSTRAINT_SOURCE_URL)
        expect(stella.source).toMatchObject({ kind: 'table', name: 'Stella', page_url: STELLA_PAGE_URL })
        expect(genoside.courses[0]).toMatchObject({
            course_hash: GENOSIDE_FIRST.courseHash,
            stage_count: STAGE_COUNT,
            active: true,
            dan: { system: 'genoside2018_sp', system_name: 'GENOSIDE 2018 SP', label: '初段', rank: 1 },
        })
    })

    test('단위를 주지 않는 코스는 dan 이 null 이다', async () => {
        const { list } = await createRoutes()
        const { data } = await (await list(get('/api/fe/courses'), undefined)).json()
        const training = data.groups[2].courses.find((course: { course_hash: string }) => course.course_hash === STELLA_TRAINING.courseHash)
        expect(training.dan).toBeNull()
    })

    test('세션이 없어도 읽고 IP 당 한도를 넘기면 429 RATE_LIMITED 이다', async () => {
        const { list } = await createRoutes()
        expect((await list(get('/api/fe/courses'), undefined)).status).toBe(200)
        for (let call = 0; call < RATE_LIMIT.PUBLIC_READ.limit; call += 1) {
            consumeRateLimit(`fe-read:${CLIENT_IP}`, RATE_LIMIT.PUBLIC_READ.limit, RATE_LIMIT.PUBLIC_READ.windowMs)
        }
        const response = await list(get('/api/fe/courses'), undefined)
        expect(response.status).toBe(429)
        expect((await response.json()).error.code).toBe('RATE_LIMITED')
    })
})

describe('GET /api/fe/courses/{courseHash}/catalog', () => {
    test('코스 하나의 묶음·출처·단위와 곡 제목을 차트 순서대로 싣는다', async () => {
        const { entry } = await createRoutes()
        const response = await entry(get('/'), entryContext(GENOSIDE_FIRST.courseHash))
        expect(response.status).toBe(200)
        const { data } = await response.json()
        expect(data).toMatchObject({
            course_hash: GENOSIDE_FIRST.courseHash,
            name: GENOSIDE_FIRST.name,
            active: true,
            group: { id: 'genoside-2018-sp', keys: 7 },
            source: { kind: 'bundled', page_url: GENOSIDE_PAGE_URL },
            dan: { system: 'genoside2018_sp', label: '初段' },
        })
        expect(data.charts).toHaveLength(STAGE_COUNT)
        expect(data.charts[FIRST_STAGE]).toMatchObject({ position: FIRST_STAGE, title: GENOSIDE_FIRST_TITLE, sha256: '' })
    })

    test('DP 코스의 묶음은 14키이다', async () => {
        const { entry } = await createRoutes()
        const { data } = await (await entry(get('/'), entryContext(GENOSIDE_DP_FIRST.courseHash))).json()
        expect(data.group).toMatchObject({ id: 'genoside-2018-dp', keys: 14 })
    })

    test('없는 코스의 catalog 는 404 IR_COURSE_NOT_FOUND 이다', async () => {
        const { entry } = await createRoutes()
        const response = await entry(get('/'), entryContext(UNKNOWN_COURSE))
        expect(response.status).toBe(404)
        expect((await response.json()).error.code).toBe('IR_COURSE_NOT_FOUND')
    })
})
