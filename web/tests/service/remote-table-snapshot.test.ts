import { describe, expect, spyOn, test } from 'bun:test'
import { createRemoteSnapshotCache, createTableLoader } from '@server/compose/table-snapshot'
import { tableEntriesQuerySchema, tableSummaryOutputSchema } from '@server/dto/table-snapshot'
import { createTableSnapshotService } from '@server/service/domain/table-snapshot/table-snapshot.service'
import {
    DAY_MS,
    FIRST_NOW,
    SATELLITE,
    STELLA,
    createMemoryStore,
    encodeJson,
    remoteEntries,
    rowOf,
    satelliteHeader,
    sha256Of,
    stellaHeader,
} from '../support/remote-table-fixture'

const ORIGIN = 'https://bms.hyuns.uk'
const STELLA_SIZE = 600
const BUNDLED_ID = 'insane1'
const UTF8_BOM = [0xef, 0xbb, 0xbf]

const createHarness = (rows = [rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE))]) => {
    const memory = createMemoryStore(rows)
    const cache = createRemoteSnapshotCache(memory.store)
    const service = createTableSnapshotService({ loadTables: createTableLoader(cache), db: { listBestByMd5: async () => [] } })
    return { ...memory, service }
}

describe('저장된 원격 표 읽기', () => {
    test('원격 표가 없으면 insane1 만 보인다', async () => {
        const { service } = createHarness([])
        expect((await service.list()).map((summary) => summary.id)).toEqual([BUNDLED_ID])
    })

    test('저장된 Stella 표는 insane1 뒤에 요약으로 나가고 레벨 순서는 수 순서이다', async () => {
        const { service } = createHarness()
        const summaries = await service.list()
        expect(summaries.map((summary) => summary.id)).toEqual([BUNDLED_ID, 'stella-st'])
        const stella = summaries[1]
        expect(stella).toMatchObject({ name: 'Stella', symbol: 'st', entry_count: STELLA_SIZE, level_count: 13 })
        expect(stella.levels.map((level) => level.level)).toEqual(Array.from({ length: 13 }, (_, index) => String(index)))
        expect(stella.standard_paths).toEqual({
            page: '/tables/stella-st',
            header: '/tables/stella-st/header.json',
            data: '/tables/stella-st/data.json',
        })
        expect(tableSummaryOutputSchema.safeParse(stella).success).toBe(true)
    })

    test('출처는 이름·제작 주체·원본 링크·받은 시각·마지막 확인·해시로 나가고 번들 표는 null 이다', async () => {
        const row = rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE), { fetchedAt: FIRST_NOW, checkedAt: FIRST_NOW + DAY_MS })
        const summaries = await createHarness([row]).service.list()
        expect(summaries[0].source).toBeNull()
        expect(summaries[1].source).toEqual({
            name: 'Stella',
            maker: 'stellabms.xyz',
            page_url: 'https://stellabms.xyz/st/table.html',
            fetched_at: FIRST_NOW,
            checked_at: FIRST_NOW + DAY_MS,
            data_sha256: row.dataSha256,
        })
    })

    test('표준 머리는 우리 주소를 가리키고 원본의 course 를 그대로 싣는다', async () => {
        const header = await createHarness().service.getStandardHeader('stella-st', ORIGIN)
        expect(header).toMatchObject({
            name: 'Stella',
            symbol: 'st',
            data_url: `${ORIGIN}/tables/stella-st/data.json`,
            course: stellaHeader().course,
        })
        expect(header?.last_update).toBe(new Date(FIRST_NOW).toISOString().slice(0, 10))
    })

    test('insane1 의 표준 머리에는 course 가 없고 키 다섯 개를 그대로 낸다', async () => {
        const header = await createHarness().service.getStandardHeader(BUNDLED_ID, ORIGIN)
        expect(Object.keys(header ?? {}).toSorted()).toEqual(['data_url', 'last_update', 'level_order', 'name', 'symbol'])
    })

    test('표준 데이터는 받은 바이트 그대로이고, 앞의 BOM 은 떼어 낸다', async () => {
        const entries = remoteEntries(STELLA_SIZE)
        const withBom = { ...rowOf(STELLA, stellaHeader(), entries), dataBytes: new Uint8Array([...UTF8_BOM, ...encodeJson(entries)]) }
        const { service } = createHarness([rowOf(STELLA, stellaHeader(), entries)])
        expect(await service.getStandardDataText('stella-st')).toBe(JSON.stringify(entries))
        expect(await createHarness([withBom]).service.getStandardDataText('stella-st')).toBe(JSON.stringify(entries))
    })

    test('Stella 의 ETag 는 data_sha256 앞 32자이다', async () => {
        const row = rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE))
        expect(await createHarness([row]).service.getDataEtag('stella-st')).toBe(`"${row.dataSha256.slice(0, 32)}"`)
        expect(sha256Of(row.dataBytes)).toBe(row.dataSha256)
    })

    test('상세·검색·폴더 가져오기가 원격 표를 읽는다', async () => {
        const { service } = createHarness()
        const detail = await service.getDetail('stella-st')
        expect(detail?.levels.reduce((sum, group) => sum + group.entries.length, 0)).toBe(STELLA_SIZE)
        const found = await service.searchEntries('stella-st', tableEntriesQuerySchema.parse({ q: 'title-7', level: '7' }))
        expect(found?.items.map((entry) => entry.title)).toContain('title-7')
        const imported = await service.loadEntriesForImport('stella-st')
        expect(imported).toHaveLength(STELLA_SIZE)
        expect(imported?.[0]).toMatchObject({ title: 'title-1', level: '1' })
    })

    test('내 기록은 원격 표의 곡 전부를 한 번에 묻는다', async () => {
        const asked: string[][] = []
        const memory = createMemoryStore([rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE))])
        const service = createTableSnapshotService({
            loadTables: createTableLoader(createRemoteSnapshotCache(memory.store)),
            db: {
                listBestByMd5: async ({ md5s }) => {
                    asked.push(md5s)
                    return []
                },
            },
        })
        expect(await service.getRecords({ tableId: 'stella-st', userId: 'user-1' })).toEqual({ table_id: 'stella-st', records: {} })
        expect(asked[0]).toHaveLength(STELLA_SIZE)
    })
})

describe('원격 표의 메모리 캐시', () => {
    test('버전이 같으면 다시 풀지 않고 버전 조회만 한다', async () => {
        const { service, calls } = createHarness()
        await service.list()
        await service.getDetail('stella-st')
        await service.getStandardDataText('stella-st')
        expect(calls.listVersions).toBe(3)
        expect(calls.loadSnapshot).toBe(1)
    })

    test('저장 내용이 바뀌면 다음 읽기에서 새 표가 보인다', async () => {
        const { service, rows, calls } = createHarness()
        expect((await service.list())[1].entry_count).toBe(STELLA_SIZE)
        rows.set(
            'stella-st',
            rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE + 7), { fetchedAt: FIRST_NOW + DAY_MS, checkedAt: FIRST_NOW + DAY_MS }),
        )
        expect((await service.list())[1].entry_count).toBe(STELLA_SIZE + 7)
        expect(calls.loadSnapshot).toBe(2)
    })

    test('확인 시각만 바뀌면 다시 풀지 않고 출처의 확인 시각만 새로 나간다', async () => {
        const { service, rows, calls } = createHarness()
        await service.list()
        rows.set('stella-st', rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE), { checkedAt: FIRST_NOW + DAY_MS }))
        const [, stella] = await service.list()
        expect(stella.source?.checked_at).toBe(FIRST_NOW + DAY_MS)
        expect(calls.loadSnapshot).toBe(1)
    })

    test('저장된 행이 사라지면 그 표도 사라진다', async () => {
        const { service, rows } = createHarness()
        await service.list()
        rows.delete('stella-st')
        expect((await service.list()).map((summary) => summary.id)).toEqual([BUNDLED_ID])
        expect(await service.getDetail('stella-st')).toBeNull()
    })

    test('두 원격 표를 정의 순서(Stella, Satellite)로 읽고 정의에 없는 표 행은 내보내지 않는다', async () => {
        const unknownRow = { ...rowOf(SATELLITE, satelliteHeader(), remoteEntries(STELLA_SIZE)), tableId: 'unknown-table' }
        const rows = [
            rowOf(SATELLITE, satelliteHeader(), remoteEntries(STELLA_SIZE, { offset: 5000 })),
            unknownRow,
            rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE)),
        ]
        const { service } = createHarness(rows)
        expect((await service.list()).map((summary) => summary.id)).toEqual([BUNDLED_ID, 'stella-st', 'stella-sl'])
    })

    test('읽을 수 없는 저장 내용은 그 표만 건너뛰고 한 번만 알린다', async () => {
        const broken = { ...rowOf(STELLA, stellaHeader(), remoteEntries(STELLA_SIZE)), dataBytes: new TextEncoder().encode('not json') }
        const { service, calls } = createHarness([broken])
        const logged = spyOn(console, 'error').mockImplementation(() => undefined)
        try {
            expect((await service.list()).map((summary) => summary.id)).toEqual([BUNDLED_ID])
            expect((await service.list()).map((summary) => summary.id)).toEqual([BUNDLED_ID])
            expect(logged.mock.calls).toEqual([['remote table copy is unreadable', { tableId: 'stella-st' }]])
        } finally {
            logged.mockRestore()
        }
        expect(calls.loadSnapshot).toBe(1)
    })
})
