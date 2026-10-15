import { describe, expect, test } from 'bun:test'
import { tableEntriesQuerySchema } from '@server/dto/table-snapshot'
import { findSnapshotTextByMd5, getSnapshotTable, getSnapshotTables } from '@server/service/domain/table-snapshot/table-snapshot.data'
import {
    createTableSnapshotService,
    groupTableByLevel,
    searchMd5sInTables,
    summariseTable,
    toStandardHeader,
    type BestRecordRow,
} from '@server/service/domain/table-snapshot/table-snapshot.service'

const TABLE_ID = 'insane1'
const ENTRY_COUNT = 1035
const LEVEL_COUNT = 26
const ORIGIN = 'https://bms.hyuns.uk'

const requireTable = () => {
    const table = getSnapshotTable(TABLE_ID)
    if (!table) throw new Error('snapshot table missing')
    return table
}

const createService = (rows: BestRecordRow[] = [], observe: (md5s: string[]) => void = () => undefined) =>
    createTableSnapshotService({
        loadTables: getSnapshotTables,
        db: {
            listBestByMd5: async ({ md5s }) => {
                observe(md5s)
                return rows.filter((row) => md5s.includes(row.md5))
            },
        },
    })

const query = (input: Record<string, string> = {}) => tableEntriesQuerySchema.parse(input)

describe('스냅샷 자료', () => {
    test('발광 표가 1035곡, 26레벨로 읽히고 source.json 의 수와 같다', () => {
        const table = requireTable()
        expect(table.entries).toHaveLength(ENTRY_COUNT)
        expect(table.source.snapshot.entries).toBe(ENTRY_COUNT)
        expect(table.source.snapshot.levels).toBe(LEVEL_COUNT)
        expect(table.header.level_order).toHaveLength(LEVEL_COUNT)
        expect(table.header.name).toBe('発狂BMS難易度表')
        expect(table.header.symbol).toBe('★')
    })

    test('md5 는 전부 소문자 16진 32자이고 겹치지 않으며 모든 레벨이 level_order 안에 있다', () => {
        const table = requireTable()
        expect(new Set(table.entries.map((entry) => entry.md5)).size).toBe(ENTRY_COUNT)
        expect(table.entries.every((entry) => /^[0-9a-f]{32}$/.test(entry.md5))).toBe(true)
        const known = new Set(table.header.level_order)
        expect(table.entries.every((entry) => known.has(entry.level))).toBe(true)
    })

    test('모르는 표 id 는 null 이다', () => {
        expect(getSnapshotTable('nope')).toBeNull()
    })

    test('md5 로 제목과 아티스트를 찾는다', () => {
        const entry = requireTable().entries[0]
        expect(findSnapshotTextByMd5(entry.md5)).toEqual({ title: entry.title, artist: entry.artist })
        expect(findSnapshotTextByMd5('0'.repeat(32))).toBeNull()
    })
})

describe('요약과 레벨 묶음', () => {
    test('레벨 수는 문서의 값(★10 이 64곡, ★25 가 2곡, ??? 가 5곡)과 같고 합계가 1035 이다', () => {
        const summary = summariseTable(requireTable())
        const counts = new Map(summary.levels.map((level) => [level.level, level.count]))
        expect(counts.get('10')).toBe(64)
        expect(counts.get('25')).toBe(2)
        expect(counts.get('???')).toBe(5)
        expect(summary.levels.reduce((sum, level) => sum + level.count, 0)).toBe(ENTRY_COUNT)
        expect(summary.levels.map((level) => level.level)).toEqual(requireTable().header.level_order)
        expect(summary.standard_paths).toEqual({
            page: '/tables/insane1',
            header: '/tables/insane1/header.json',
            data: '/tables/insane1/data.json',
        })
    })

    test('레벨 묶음은 level_order 순서이고 곡 수가 요약과 같다', () => {
        const detail = groupTableByLevel(requireTable())
        expect(detail.levels.map((group) => group.level)).toEqual(requireTable().header.level_order)
        expect(detail.levels.every((group) => group.entries.every((entry) => entry.level === group.level))).toBe(true)
        expect(detail.levels.reduce((sum, group) => sum + group.entries.length, 0)).toBe(ENTRY_COUNT)
    })

    test('list 는 등록된 표마다 요약 하나이다', () => {
        expect(
            createService()
                .list()
                .map((summary) => summary.id),
        ).toEqual([TABLE_ID])
    })
})

describe('표준 표 형식', () => {
    test('header 는 R-BMS 가 읽는 필드를 갖고 data_url 이 요청 origin 의 절대 주소이다', () => {
        const header = toStandardHeader(requireTable(), ORIGIN)
        expect(header.data_url).toBe('https://bms.hyuns.uk/tables/insane1/data.json')
        expect(header.name).toBe('発狂BMS難易度表')
        expect(header.symbol).toBe('★')
        expect(header.level_order).toHaveLength(LEVEL_COUNT)
        expect(Object.keys(header).toSorted()).toEqual(['data_url', 'last_update', 'level_order', 'name', 'symbol'])
        expect(toStandardHeader(requireTable(), 'http://localhost:3102').data_url).toBe('http://localhost:3102/tables/insane1/data.json')
    })

    test('data 본문은 표준 필드를 갖되 다른 사이트의 주소는 담지 않는다', () => {
        const text = createService().getStandardDataText(TABLE_ID)
        const parsed: Record<string, string>[] = JSON.parse(text ?? '[]')
        expect(parsed).toHaveLength(ENTRY_COUNT)
        expect(Object.keys(parsed[0]).sort()).toEqual(['artist', 'comment', 'level', 'lr2_bmsid', 'md5', 'name_diff', 'title', 'url', 'url_diff'])
        expect(text).not.toMatch(/https?:\/\/|www\./)
        expect(parsed.every((row) => row.url === '' && row.url_diff === '' && row.name_diff === '' && row.comment === '')).toBe(true)
        expect(parsed.map((entry) => entry.md5)).toEqual(requireTable().entries.map((entry) => entry.md5))
    })

    test('같은 본문을 두 번 만들지 않고 모르는 표는 null 이다', () => {
        const service = createService()
        expect(service.getStandardDataText(TABLE_ID)).toBe(service.getStandardDataText(TABLE_ID))
        expect(service.getStandardDataText('nope')).toBeNull()
        expect(service.getStandardHeader('nope', ORIGIN)).toBeNull()
        expect(service.getDataEtag('nope')).toBeNull()
    })

    test('ETag 는 정규화한 data 파일의 해시 앞 32자이다', () => {
        const etag = createService().getDataEtag(TABLE_ID)
        expect(etag).toBe(`"${requireTable().source.snapshot.data_file_sha256.slice(0, 32)}"`)
    })
})

describe('곡 검색', () => {
    test('제목은 대소문자를 구분하지 않고 전각도 같은 글자로 본다', () => {
        const service = createService()
        const lower = service.searchEntries(TABLE_ID, query({ q: 'born [29another]' }))
        const fullwidth = service.searchEntries(TABLE_ID, query({ q: 'ＢＯＲＮ [29ANOTHER]' }))
        expect(lower?.items.map((entry) => entry.title)).toContain('Born [29Another]')
        expect(fullwidth?.total).toBe(lower?.total)
    })

    test('아티스트로도 찾고 레벨로 좁힌다', () => {
        const service = createService()
        const byArtist = service.searchEntries(TABLE_ID, query({ q: 'ninja action team' }))
        expect(byArtist?.items.some((entry) => entry.artist.includes('Ninja Action Team'))).toBe(true)
        const byLevel = service.searchEntries(TABLE_ID, query({ level: '25' }))
        expect(byLevel?.total).toBe(2)
    })

    test('md5 앞 6자 이상이면 그 곡을 찾는다', () => {
        const entry = requireTable().entries[10]
        const found = createService().searchEntries(TABLE_ID, query({ q: entry.md5.slice(0, 8) }))
        expect(found?.items.map((item) => item.md5)).toContain(entry.md5)
    })

    test('쪽 나누기와 맞는 곡 없음', () => {
        const service = createService()
        const page = service.searchEntries(TABLE_ID, query({ limit: '10', page: '2' }))
        expect(page?.items).toHaveLength(10)
        expect(page?.total).toBe(ENTRY_COUNT)
        expect(service.searchEntries(TABLE_ID, query({ q: 'zzzz-no-such-title-zzzz' }))).toEqual({ items: [], total: 0 })
        expect(service.searchEntries('nope', query())).toBeNull()
    })

    test('searchMd5sInTables 는 개수 상한을 지킨다', () => {
        expect(searchMd5sInTables(getSnapshotTables(), 'a', 5)).toHaveLength(5)
        expect(searchMd5sInTables(getSnapshotTables(), '   ')).toEqual([])
    })
})

describe('내 기록', () => {
    const entries = requireTable().entries

    test('표의 md5 전부를 한 번에 묻고 기록이 있는 곡만 램프 이름으로 돌려준다', async () => {
        const asked: string[][] = []
        const service = createService(
            [
                {
                    md5: entries[0].md5,
                    clear: 6,
                    exScore: 1234,
                    minbp: 7,
                    maxCombo: 321,
                    totalNotes: 800,
                    playedAt: 1_760_000_000_000,
                    scratchAuto: true,
                },
                { md5: '0'.repeat(32), clear: 4, exScore: 1, minbp: 1, maxCombo: 1, totalNotes: 1, playedAt: 1, scratchAuto: false },
            ],
            (md5s) => asked.push(md5s),
        )
        const records = await service.getRecords({ tableId: TABLE_ID, userId: 'user-1' })
        expect(asked).toHaveLength(1)
        expect(asked[0]).toHaveLength(ENTRY_COUNT)
        expect(records?.table_id).toBe(TABLE_ID)
        expect(Object.keys(records?.records ?? {})).toEqual([entries[0].md5])
        expect(records?.records[entries[0].md5]).toEqual({
            clear: 'Hard',
            ex_score: 1234,
            minbp: 7,
            max_combo: 321,
            total_notes: 800,
            played_at: 1_760_000_000_000,
            scratch_auto: true,
        })
    })

    test('모르는 표는 null 이고 DB 를 부르지 않는다', async () => {
        const asked: string[][] = []
        expect(await createService([], (md5s) => asked.push(md5s)).getRecords({ tableId: 'nope', userId: 'user-1' })).toBeNull()
        expect(asked).toHaveLength(0)
    })

    test('1035곡 전부 기록이 있어도 응답 JSON 이 200KB 안쪽이다', async () => {
        const rows = entries.map((entry) => ({
            md5: entry.md5,
            clear: 10,
            exScore: 99_999,
            minbp: 99_999,
            maxCombo: 99_999,
            totalNotes: 99_999,
            playedAt: 1_760_000_000_000,
            scratchAuto: true,
        }))
        const records = await createService(rows).getRecords({ tableId: TABLE_ID, userId: 'user-1' })
        expect(JSON.stringify({ success: true, data: records }).length).toBeLessThan(200 * 1024)
    })
})

describe('FE 상세 응답의 크기', () => {
    test('1035곡 전체 상세가 gzip 전 400KB 안쪽이다', () => {
        const size = JSON.stringify({ success: true, data: groupTableByLevel(requireTable()) }).length
        expect(size).toBeLessThan(400 * 1024)
    })
})
