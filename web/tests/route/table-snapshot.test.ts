import { beforeEach, describe, expect, test } from 'bun:test'
import { createRemoteSnapshotCache, createTableLoader } from '@server/compose/table-snapshot'
import type { AuthUser } from '@server/lib/auth-context'
import { resetRateLimits } from '@server/lib/with-rate-limit'
import { createTableSnapshotRoutes } from '@server/route/table-snapshot'
import { getSnapshotTables } from '@server/service/domain/table-snapshot/table-snapshot.data'
import { createTableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import { STELLA, createMemoryStore, remoteEntries, rowOf, stellaHeader } from '../support/remote-table-fixture'

const ORIGIN = 'https://bms.hyuns.uk'
const ALICE: AuthUser = { id: 'user-alice', loginId: 'alice', name: 'Alice', role: 'user' }
const tableContext = (tableId: string) => ({ params: Promise.resolve({ tableId }) })

const createRoutes = () =>
    createTableSnapshotRoutes(() => ({
        authService: { resolveSessionUser: async (request) => (request.headers.get('x-session-user') === 'alice' ? ALICE : null) },
        tableSnapshotService: createTableSnapshotService({
            loadTables: async () => getSnapshotTables(),
            db: {
                listBestByMd5: async ({ md5s }) => [
                    {
                        md5: md5s[0],
                        clear: 5,
                        assistGrade: 0,
                        exScore: 100,
                        minbp: 3,
                        maxCombo: 50,
                        totalNotes: 80,
                        playedAt: 1_760_000_000_000,
                        scratchAuto: false,
                    },
                ],
            },
        }),
    }))

beforeEach(() => resetRateLimits())

describe('표준 표 형식 header.json', () => {
    test('JSON 객체를 CORS 와 캐시 헤더와 함께 내고 data_url 이 요청 origin 의 절대 주소이다', async () => {
        const response = await createRoutes().standardHeader(new Request(`${ORIGIN}/tables/insane1/header.json`), tableContext('insane1'))
        expect(response.status).toBe(200)
        expect(response.headers.get('content-type')).toBe('application/json; charset=utf-8')
        expect(response.headers.get('access-control-allow-origin')).toBe('*')
        expect(response.headers.get('cache-control')).toContain('public')
        expect(response.headers.get('cache-control')).toContain('s-maxage=')
        const text = await response.text()
        expect(text.trimStart().startsWith('{')).toBe(true)
        const header = JSON.parse(text)
        expect(header).toMatchObject({ name: '発狂BMS難易度表', symbol: '★', data_url: 'https://bms.hyuns.uk/tables/insane1/data.json' })
        expect(header.level_order).toHaveLength(26)
    })

    test('다른 origin 에서 받으면 그 origin 으로 data_url 을 만든다', async () => {
        const response = await createRoutes().standardHeader(new Request('http://localhost:3102/tables/insane1/header.json'), tableContext('insane1'))
        expect((await response.json()).data_url).toBe('http://localhost:3102/tables/insane1/data.json')
    })

    test('모르는 표나 모양이 틀린 id 는 404 IR_TABLE_NOT_FOUND 이다', async () => {
        for (const tableId of ['nope', '../insane1', 'INSANE1']) {
            const response = await createRoutes().standardHeader(new Request(`${ORIGIN}/tables/x/header.json`), tableContext(tableId))
            expect(response.status).toBe(404)
            expect((await response.json()).error.code).toBe('IR_TABLE_NOT_FOUND')
        }
    })
})

describe('표준 표 형식 data.json', () => {
    test('1034곡 배열을 내고 첫 글자가 [ 이다', async () => {
        const response = await createRoutes().standardData(new Request(`${ORIGIN}/tables/insane1/data.json`), tableContext('insane1'))
        expect(response.status).toBe(200)
        expect(response.headers.get('content-type')).toBe('application/json; charset=utf-8')
        expect(response.headers.get('access-control-allow-origin')).toBe('*')
        const text = await response.text()
        expect(text.startsWith('[')).toBe(true)
        expect(JSON.parse(text)).toHaveLength(1034)
    })

    test('If-None-Match 가 ETag 와 같으면 본문 없는 304 이다', async () => {
        const first = await createRoutes().standardData(new Request(`${ORIGIN}/tables/insane1/data.json`), tableContext('insane1'))
        const etag = first.headers.get('etag')
        expect(etag).toMatch(/^"[0-9a-f]{32}"$/)
        const second = await createRoutes().standardData(
            new Request(`${ORIGIN}/tables/insane1/data.json`, { headers: { 'if-none-match': etag ?? '' } }),
            tableContext('insane1'),
        )
        expect(second.status).toBe(304)
        expect(await second.text()).toBe('')
    })

    test('모르는 표는 404 이다', async () => {
        const response = await createRoutes().standardData(new Request(`${ORIGIN}/tables/nope/data.json`), tableContext('nope'))
        expect(response.status).toBe(404)
    })
})

describe('클라이언트 주소 해석 규칙', () => {
    test('루트 상대 bmstable 메타와 절대 data_url 이 정본 주소에서 풀린다', async () => {
        const page = new URL('https://bms.hyuns.uk/tables/insane1')
        const headerUrl = new URL('/tables/insane1/header.json', page)
        expect(headerUrl.href).toBe('https://bms.hyuns.uk/tables/insane1/header.json')
        const header = await (await createRoutes().standardHeader(new Request(headerUrl), tableContext('insane1'))).json()
        expect(new URL(header.data_url, headerUrl).href).toBe('https://bms.hyuns.uk/tables/insane1/data.json')
        const trailing = new URL('/tables/insane1/header.json', new URL('https://bms.hyuns.uk/tables/insane1/'))
        expect(trailing.href).toBe(headerUrl.href)
    })
})

describe('FE 표 API', () => {
    test('목록과 상세는 봉투와 공개 캐시 헤더로 답한다', async () => {
        const list = await createRoutes().list(new Request(`${ORIGIN}/api/fe/tables`), undefined)
        expect(list.status).toBe(200)
        expect(list.headers.get('cache-control')).toContain('s-maxage=')
        expect((await list.json()).data[0]).toMatchObject({ id: 'insane1', entry_count: 1034, level_count: 26 })
        const detail = await createRoutes().detail(new Request(`${ORIGIN}/api/fe/tables/insane1`), tableContext('insane1'))
        const body = await detail.json()
        expect(body.success).toBe(true)
        expect(body.data.levels).toHaveLength(26)
    })

    test('곡 검색은 쪽 나누기 정보를 담고 limit 상한을 검증한다', async () => {
        const ok = await createRoutes().entries(new Request(`${ORIGIN}/api/fe/tables/insane1/entries?level=25`), tableContext('insane1'))
        const body = await ok.json()
        expect(body.data).toHaveLength(2)
        expect(body.pagination).toMatchObject({ page: 1, limit: 50, total: 2, totalPages: 1 })
        const tooMany = await createRoutes().entries(new Request(`${ORIGIN}/api/fe/tables/insane1/entries?limit=101`), tableContext('insane1'))
        expect(tooMany.status).toBe(400)
        const unknown = await createRoutes().entries(new Request(`${ORIGIN}/api/fe/tables/nope/entries`), tableContext('nope'))
        expect(unknown.status).toBe(404)
    })

    test('내 기록은 세션이 없으면 401 이고 있으면 private 캐시로 답한다', async () => {
        const anonymous = await createRoutes().records(new Request(`${ORIGIN}/api/fe/tables/insane1/records`), tableContext('insane1'))
        expect(anonymous.status).toBe(401)
        const response = await createRoutes().records(
            new Request(`${ORIGIN}/api/fe/tables/insane1/records`, { headers: { 'x-session-user': 'alice' } }),
            tableContext('insane1'),
        )
        expect(response.status).toBe(200)
        expect(response.headers.get('cache-control')).toBe('private, no-store')
        const body = await response.json()
        expect(body.data.table_id).toBe('insane1')
        expect(Object.values(body.data.records)[0]).toMatchObject({ clear: 'Normal', ex_score: 100, total_notes: 80 })
    })

    test('내 기록은 분당 30회를 넘으면 429 이고 모르는 표는 404 이다', async () => {
        const routes = createRoutes()
        const request = () => new Request(`${ORIGIN}/api/fe/tables/insane1/records`, { headers: { 'x-session-user': 'alice' } })
        let last = 0
        for (let n = 0; n < 31; n += 1) last = (await routes.records(request(), tableContext('insane1'))).status
        expect(last).toBe(429)
        resetRateLimits()
        const unknown = await routes.records(new Request(`${ORIGIN}/x`, { headers: { 'x-session-user': 'alice' } }), tableContext('nope'))
        expect(unknown.status).toBe(404)
    })
})

describe('저장된 Stella 표의 표준 형식', () => {
    const STELLA_SIZE = 600
    const stored = rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE))

    const createStellaRoutes = () =>
        createTableSnapshotRoutes(() => ({
            authService: { resolveSessionUser: async () => null },
            tableSnapshotService: createTableSnapshotService({
                loadTables: createTableLoader(createRemoteSnapshotCache(createMemoryStore([stored]).store)),
                db: { listBestByMd5: async () => [] },
            }),
        }))

    test('header.json 은 우리 주소의 data_url 과 원본의 course 를 싣는다', async () => {
        const response = await createStellaRoutes().standardHeader(new Request(`${ORIGIN}/tables/stella-st/header.json`), tableContext('stella-st'))
        expect(response.status).toBe(200)
        expect(response.headers.get('access-control-allow-origin')).toBe('*')
        expect(await response.json()).toMatchObject({
            name: 'Stella',
            symbol: 'st',
            data_url: `${ORIGIN}/tables/stella-st/data.json`,
            course: stellaHeader().course,
        })
    })

    test('data.json 은 받은 본문 그대로이고 ETag 가 같으면 304 이다', async () => {
        const first = await createStellaRoutes().standardData(new Request(`${ORIGIN}/tables/stella-st/data.json`), tableContext('stella-st'))
        expect(await first.text()).toBe(new TextDecoder().decode(stored.dataBytes))
        const etag = first.headers.get('etag')
        expect(etag).toBe(`"${stored.dataSha256.slice(0, 32)}"`)
        const second = await createStellaRoutes().standardData(
            new Request(`${ORIGIN}/tables/stella-st/data.json`, { headers: { 'if-none-match': etag ?? '' } }),
            tableContext('stella-st'),
        )
        expect(second.status).toBe(304)
    })

    test('FE 목록과 상세에도 나가고 목록의 요약에 출처가 실린다', async () => {
        const list = await createStellaRoutes().list(new Request(`${ORIGIN}/api/fe/tables`), undefined)
        const body = await list.json()
        expect(body.data.map((table: { id: string }) => table.id)).toEqual(['insane1', 'stella-st'])
        expect(body.data[0].source).toBeNull()
        expect(body.data[1].source).toMatchObject({ name: 'Stella', maker: 'stellabms.xyz' })
        const detail = await createStellaRoutes().detail(new Request(`${ORIGIN}/api/fe/tables/stella-st`), tableContext('stella-st'))
        expect((await detail.json()).data.table.entry_count).toBe(STELLA_SIZE)
    })
})
